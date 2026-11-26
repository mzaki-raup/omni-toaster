//! Reminders, their snoozes and the record of what fired
//! (`docs/specs/reminders.md` §4, §5, §9).
//!
//! `trigger_json` and `recurrence_json` are read into typed Rust, and a field
//! this build does not know is refused rather than ignored: a reminder whose
//! trigger cannot be understood must not fire at a guessed time. `next_fire_at`
//! belongs to the scheduler — this module stores it but only
//! [`Store::set_next_fire_at`] writes it, and an edit clears it so a stale time
//! is never trusted.
//!
//! Instants are stored as UTC text, `2026-11-19T05:00:00.000Z`, the same shape
//! SQLite's own `strftime('%Y-%m-%dT%H:%M:%fZ')` writes, so they sort and
//! compare as text.

use jiff::{SignedDuration, Timestamp, civil};
use rusqlite::{OptionalExtension as _, Row, params};
use serde::{Deserialize, Serialize};

use crate::{Store, StoreError};

/// How long `notification_log` keeps a row (`docs/05-data-model-and-storage.md`).
pub const NOTIFICATION_LOG_DAYS: i64 = 90;

/// When a reminder fires, in the local wall clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Trigger {
    /// At the exact time.
    At { at: civil::DateTime },
    /// Ahead of the time, once per offset; each offset is its own fire.
    OffsetBefore {
        at: civil::DateTime,
        offsets: Vec<Offset>,
    },
    /// A fatigue reminder: every so many minutes of the working day — eye
    /// break, posture, drink water (`docs/specs/reminders.md` §3).
    Every { minutes: u32 },
    /// The work-hour limit: once, so many minutes after the working day
    /// began.
    WorkHours { minutes: u32 },
    /// A prayer: at each day's time for it, which the scheduler is given
    /// rather than computes (`docs/11-prayer-and-azan.md` §9). `prayer` is one
    /// of [`PRAYERS`].
    Prayer { prayer: String },
    /// A reminder so many minutes **before** a prayer, optional per prayer
    /// (`docs/11-prayer-and-azan.md` §9). It follows the prayer's time: when the
    /// prayer moves, so does it.
    PrayerBefore { prayer: String, minutes: u32 },
}

/// What the user wrote for a prayer's template; `None` is the built-in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrayerTemplateEdit {
    pub title: Option<String>,
    pub body: Option<String>,
    /// `#rrggbb`.
    pub accent: Option<String>,
    /// A library picture's `sha256`.
    pub image: Option<String>,
    /// `en-US` or `ms-MY`: the language the text was typed in.
    pub authored_lang: &'static str,
    /// `chime`, `azan`, `file` or `none`; `None` is the chime.
    pub sound_mode: Option<String>,
    /// An imported clip's `sha256`, for `file`.
    pub sound: Option<String>,
}

/// The prayers a managed reminder can be for, by the key the scheduler is
/// given their times under, in the order of the day.
pub const PRAYERS: [&str; 7] = [
    "fajr", "syuruk", "dhuhr", "jumuah", "asr", "maghrib", "isha",
];

impl Trigger {
    /// The `trigger_type` column's value.
    fn type_name(&self) -> &'static str {
        match self {
            Self::At { .. } => "at",
            Self::OffsetBefore { .. } => "offset_before",
            Self::Every { .. } => "every",
            Self::WorkHours { .. } => "work_hours",
            Self::Prayer { .. } => "prayer",
            Self::PrayerBefore { .. } => "prayer_before",
        }
    }

    /// Whether this is one of the fatigue schedules, which count from the
    /// working day rather than from a date, and are managed in Settings ▸
    /// Reminders' Fatigue Reminders rather than in the editor.
    pub fn is_fatigue(&self) -> bool {
        matches!(self, Self::Every { .. } | Self::WorkHours { .. })
    }

    /// Whether this is a prayer's reminder, which the app manages from the
    /// Prayer settings and the list in Settings ▸ Reminders does not show.
    pub fn is_prayer(&self) -> bool {
        matches!(self, Self::Prayer { .. } | Self::PrayerBefore { .. })
    }

    /// Whether this is a reminder ahead of a prayer rather than the prayer.
    pub fn is_pre_prayer(&self) -> bool {
        matches!(self, Self::PrayerBefore { .. })
    }
}

/// The fatigue kinds: each one's type key, whether it repeats (`Every`) or
/// comes once (`WorkHours`), and its default in minutes
/// (`docs/specs/reminders.md` §3): an eye break every 20 minutes, the
/// posture every 30, a drink every hour, and the work-hour limit after 8
/// hours.
pub const FATIGUE: [(&str, bool, u32); 4] = [
    ("reminder-fatigue-eye", true, 20),
    ("reminder-fatigue-posture", true, 30),
    ("reminder-fatigue-hydration", true, 60),
    ("reminder-fatigue-work-hours", false, 8 * 60),
];

/// An offset before the time: 1 week, 2 days, 1 hour, 15 minutes, or custom.
/// Kept in the unit the user chose, so the editor shows "2 days", not "2880
/// minutes".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offset {
    pub amount: u32,
    pub unit: OffsetUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OffsetUnit {
    Minutes,
    Hours,
    Days,
    Weeks,
}

/// How a reminder repeats. A reminder that fires once has none.
///
/// Every *N* days, weeks, months or years; weekly may name its weekdays. There
/// is no expression language (`docs/specs/reminders.md` §4, §14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recurrence {
    pub every: u32,
    pub unit: RecurrenceUnit,
    /// Only for weekly; empty means the weekday of the trigger's date.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weekdays: Vec<Weekday>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurrenceUnit {
    Days,
    Weeks,
    Months,
    Years,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

/// `reminder.status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Active,
    Snoozed,
    Done,
    Cancelled,
}

impl Status {
    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "active" => Self::Active,
            "snoozed" => Self::Snoozed,
            "done" => Self::Done,
            "cancelled" => Self::Cancelled,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Snoozed => "snoozed",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }
}

/// What the user wrote. Title and body are stored exactly as typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderDraft {
    /// The template, a `category` row; `None` for the global default.
    pub category_id: Option<i64>,
    pub title: String,
    pub body: Option<String>,
    /// The language the title and body were typed in: `en-US` or `ms-MY`.
    pub authored_lang: String,
    pub trigger: Trigger,
    pub recurrence: Option<Recurrence>,
}

/// A stored reminder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reminder {
    pub id: i64,
    pub draft: ReminderDraft,
    /// Written by the scheduler only.
    pub next_fire_at: Option<Timestamp>,
    pub status: Status,
}

/// `notification_log.action`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationAction {
    Shown,
    Snoozed,
    Dismissed,
    Opened,
    Missed,
}

impl NotificationAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Shown => "shown",
            Self::Snoozed => "snoozed",
            Self::Dismissed => "dismissed",
            Self::Opened => "opened",
            Self::Missed => "missed",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "shown" => Self::Shown,
            "snoozed" => Self::Snoozed,
            "dismissed" => Self::Dismissed,
            "opened" => Self::Opened,
            "missed" => Self::Missed,
            _ => return None,
        })
    }
}

/// Why a reminder could not be stored or read.
#[derive(Debug, thiserror::Error)]
pub enum ReminderError {
    #[error(transparent)]
    Store(#[from] StoreError),

    /// A stored column this build cannot understand — a field it does not
    /// know, a missing one, or text that is not JSON at all.
    #[error("reminder {id}'s {column} is not one this build understands: {detail}")]
    Malformed {
        id: i64,
        column: &'static str,
        detail: String,
    },

    /// Writing the JSON failed. These types cannot fail to serialize — no
    /// maps, no hand-written `Serialize` — so this is carried, not expected.
    #[error("a reminder could not be written as JSON: {0}")]
    Encode(#[from] serde_json::Error),
}

impl From<rusqlite::Error> for ReminderError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Store(StoreError::Query(error))
    }
}

/// The built-in reminder types' keys, in the order migration 003 seeds them
/// (`docs/specs/reminders.md` §3). Each is a template's `type_key` and the
/// Fluent key of the type's name; `<key>-body` is its default toast text.
pub const REMINDER_TYPES: &[&str] = &[
    "reminder-event",
    "reminder-task",
    "reminder-anniversary",
    "reminder-fatigue-eye",
    "reminder-fatigue-posture",
    "reminder-fatigue-hydration",
    "reminder-fatigue-work-hours",
    "reminder-timer",
    "reminder-alarm",
    "reminder-payment",
    "reminder-appointment",
    "reminder-custom",
];

/// A template — a `category` row — as a toast needs it. The custom text is
/// the user's, stored as typed; `None` means the built-in text in the active
/// language (`docs/specs/toast.md` §8).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Template {
    pub id: i64,
    /// The built-in reminder type, `reminder-event` and so on; `None` for a
    /// template that is not a reminder type's.
    pub type_key: Option<String>,
    /// The prayer this template is for, one of [`PRAYERS`]; `None` for any
    /// other template.
    pub prayer_key: Option<String>,
    pub custom_title: Option<String>,
    pub custom_body: Option<String>,
    /// `#rrggbb`, as the editor stores it.
    pub accent_color: Option<String>,
    /// The template's picture, as a relative path under the data folder.
    pub image: Option<String>,
    /// A prayer's sound: `chime` (the default, also `None`), `azan`, `file` or
    /// `none`.
    pub sound_mode: Option<String>,
    /// The imported clip for `file`, as a relative path under the data folder.
    pub sound: Option<String>,
}

/// An instant as the database stores it.
pub fn stamp(when: Timestamp) -> String {
    when.strftime("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// The raw columns of one row, before the JSON and the status are checked.
struct RawRow {
    id: i64,
    category_id: Option<i64>,
    title: String,
    body: Option<String>,
    authored_lang: Option<String>,
    trigger_json: String,
    recurrence_json: Option<String>,
    next_fire_at: Option<String>,
    status: String,
}

impl RawRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            category_id: row.get(1)?,
            title: row.get(2)?,
            body: row.get(3)?,
            authored_lang: row.get(4)?,
            trigger_json: row.get(5)?,
            recurrence_json: row.get(6)?,
            next_fire_at: row.get(7)?,
            status: row.get(8)?,
        })
    }

    fn into_reminder(self) -> Result<Reminder, ReminderError> {
        let id = self.id;
        let malformed = |column, detail: String| ReminderError::Malformed { id, column, detail };

        let trigger = serde_json::from_str(&self.trigger_json)
            .map_err(|e| malformed("trigger_json", e.to_string()))?;
        let recurrence = self
            .recurrence_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(|e| malformed("recurrence_json", e.to_string()))?;
        let next_fire_at = self
            .next_fire_at
            .as_deref()
            .map(str::parse::<Timestamp>)
            .transpose()
            .map_err(|e| malformed("next_fire_at", e.to_string()))?;
        let status = Status::parse(&self.status)
            .ok_or_else(|| malformed("status", format!("unknown status {:?}", self.status)))?;

        Ok(Reminder {
            id,
            draft: ReminderDraft {
                category_id: self.category_id,
                title: self.title,
                body: self.body,
                authored_lang: self.authored_lang.unwrap_or_default(),
                trigger,
                recurrence,
            },
            next_fire_at,
            status,
        })
    }
}

/// The JSON for a draft's trigger and recurrence.
fn encode(draft: &ReminderDraft) -> Result<(String, Option<String>), ReminderError> {
    let trigger = serde_json::to_string(&draft.trigger)?;
    let recurrence = draft
        .recurrence
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    Ok((trigger, recurrence))
}

impl Store {
    /// Store a new reminder and return its id. It has no fire time until the
    /// scheduler gives it one.
    pub fn create_reminder(&self, draft: &ReminderDraft) -> Result<i64, ReminderError> {
        let (trigger, recurrence) = encode(draft)?;
        self.conn.execute(
            "INSERT INTO reminder (category_id, title, body, authored_lang, trigger_type, \
             trigger_json, recurrence_json, status, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', \
             strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params![
                draft.category_id,
                draft.title,
                draft.body,
                draft.authored_lang,
                draft.trigger.type_name(),
                trigger,
                recurrence,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// One reminder, or `None` if there is no such id. A row whose JSON this
    /// build cannot read is an error, not a guess.
    pub fn reminder(&self, id: i64) -> Result<Option<Reminder>, ReminderError> {
        let raw = self
            .conn
            .query_row(
                "SELECT id, category_id, title, body, authored_lang, trigger_json, \
                 recurrence_json, next_fire_at, status FROM reminder WHERE id = ?1",
                params![id],
                RawRow::read,
            )
            .optional()?;
        raw.map(RawRow::into_reminder).transpose()
    }

    /// Every reminder, oldest first. A row this build cannot read is left out
    /// with a warning in the log, so one bad row does not hide the rest.
    pub fn reminders(&self) -> Result<Vec<Reminder>, ReminderError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category_id, title, body, authored_lang, trigger_json, \
             recurrence_json, next_fire_at, status FROM reminder ORDER BY id",
        )?;
        let rows = stmt.query_map([], RawRow::read)?;
        let mut reminders = Vec::new();
        for raw in rows {
            match raw?.into_reminder() {
                Ok(reminder) => reminders.push(reminder),
                Err(error) => tracing::warn!(%error, "a reminder was left out of the list"),
            }
        }
        Ok(reminders)
    }

    /// Replace what the user wrote. Clears the fire time, so the scheduler
    /// recomputes it rather than firing at the old one. Returns whether the
    /// reminder existed.
    pub fn update_reminder(&self, id: i64, draft: &ReminderDraft) -> Result<bool, ReminderError> {
        let (trigger, recurrence) = encode(draft)?;
        let changed = self.conn.execute(
            "UPDATE reminder SET category_id = ?2, title = ?3, body = ?4, authored_lang = ?5, \
             trigger_type = ?6, trigger_json = ?7, recurrence_json = ?8, next_fire_at = NULL, \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
            params![
                id,
                draft.category_id,
                draft.title,
                draft.body,
                draft.authored_lang,
                draft.trigger.type_name(),
                trigger,
                recurrence,
            ],
        )?;
        Ok(changed == 1)
    }

    /// Keep one managed reminder for each prayer in `enabled` — keys from
    /// [`PRAYERS`] — and none for the others. A prayer already there is left
    /// alone, so its stored next time and its history survive a restart.
    /// Returns whether anything was created or deleted.
    pub fn sync_prayer_reminders(&self, enabled: &[&str]) -> Result<bool, ReminderError> {
        let mut changed = false;
        let mut present = Vec::new();
        for reminder in self.reminders()? {
            let Trigger::Prayer { prayer } = &reminder.draft.trigger else {
                continue;
            };
            if enabled.contains(&prayer.as_str()) && !present.contains(prayer) {
                present.push(prayer.clone());
            } else {
                // Not enabled, or a duplicate: gone.
                self.delete_reminder(reminder.id)?;
                changed = true;
            }
        }
        for prayer in enabled.iter().filter(|p| PRAYERS.contains(p)) {
            if present.iter().any(|have| have == prayer) {
                continue;
            }
            self.create_reminder(&ReminderDraft {
                category_id: None,
                // The name is resolved from the key when the toast is built,
                // in the language then in force, never stored.
                title: String::new(),
                body: None,
                authored_lang: "en-US".to_owned(),
                trigger: Trigger::Prayer {
                    prayer: (*prayer).to_owned(),
                },
                recurrence: None,
            })?;
            changed = true;
        }
        self.link_prayer_templates()?;
        Ok(changed)
    }

    /// Keep one reminder `minutes` before each prayer in `enabled`, and none for
    /// the others. A prayer already there keeps its row; one whose lead changed
    /// is updated in place. Returns whether anything changed.
    pub fn sync_pre_prayer_reminders(
        &self,
        enabled: &[&str],
        minutes: u32,
    ) -> Result<bool, ReminderError> {
        let mut changed = false;
        let mut present: Vec<String> = Vec::new();
        for reminder in self.reminders()? {
            let Trigger::PrayerBefore {
                prayer,
                minutes: have,
            } = &reminder.draft.trigger
            else {
                continue;
            };
            if !enabled.contains(&prayer.as_str()) || present.contains(prayer) {
                self.delete_reminder(reminder.id)?;
                changed = true;
                continue;
            }
            present.push(prayer.clone());
            if *have != minutes {
                let mut draft = reminder.draft.clone();
                draft.trigger = Trigger::PrayerBefore {
                    prayer: prayer.clone(),
                    minutes,
                };
                self.update_reminder(reminder.id, &draft)?;
                changed = true;
            }
        }
        for prayer in enabled.iter().filter(|p| PRAYERS.contains(p)) {
            if present.iter().any(|have| have == prayer) {
                continue;
            }
            self.create_reminder(&ReminderDraft {
                category_id: None,
                title: String::new(),
                body: None,
                authored_lang: "en-US".to_owned(),
                trigger: Trigger::PrayerBefore {
                    prayer: (*prayer).to_owned(),
                    minutes,
                },
                recurrence: None,
            })?;
            changed = true;
        }
        self.link_prayer_templates()?;
        Ok(changed)
    }

    /// Delete a reminder. Its snoozes go with it (`ON DELETE CASCADE`); its
    /// history rows stay, unlinked, until they age out. Returns whether it
    /// existed.
    pub fn delete_reminder(&self, id: i64) -> Result<bool, StoreError> {
        let changed = self
            .conn
            .execute("DELETE FROM reminder WHERE id = ?1", params![id])?;
        Ok(changed == 1)
    }

    /// The scheduler's write: the next instant this reminder fires, or `None`
    /// when it has no more.
    pub fn set_next_fire_at(&self, id: i64, next: Option<Timestamp>) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE reminder SET next_fire_at = ?2 WHERE id = ?1",
            params![id, next.map(stamp)],
        )?;
        Ok(())
    }

    /// Set a reminder's status. The scheduler marks a reminder `Done` when it
    /// has fired for the last time, so a clock set back cannot fire it again.
    pub fn set_status(&self, id: i64, status: Status) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE reminder SET status = ?2, \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
            params![id, status.as_str()],
        )?;
        Ok(())
    }

    /// Record a snooze. `count` is this snooze's place in its fire's chain:
    /// 1 for the first snooze of a fire, 2 for a snooze of that snooze, and so
    /// on — a recurring reminder's next occurrence starts again at 1
    /// (`docs/specs/reminders.md` §5).
    pub fn record_snooze(
        &self,
        reminder_id: i64,
        until: Timestamp,
        count: u32,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO snooze_log (reminder_id, snoozed_until, count) VALUES (?1, ?2, ?3)",
            params![reminder_id, stamp(until), count],
        )?;
        Ok(())
    }

    /// A reminder's latest snooze: when it ends and its count. While the
    /// reminder's status is `snoozed`, this is the snooze still to fire, which
    /// is how a snooze survives a restart.
    pub fn latest_snooze(&self, reminder_id: i64) -> Result<Option<(Timestamp, u32)>, StoreError> {
        let row: Option<(String, i64)> = self
            .conn
            .query_row(
                "SELECT snoozed_until, count FROM snooze_log WHERE reminder_id = ?1                  ORDER BY id DESC LIMIT 1",
                params![reminder_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        Ok(row.and_then(|(until, count)| {
            let until = until.parse().ok()?;
            Some((until, u32::try_from(count).unwrap_or(u32::MAX)))
        }))
    }

    /// A template by its `category` id, with its picture's relative path, or
    /// `None` if there is no such row.
    pub fn template(&self, id: i64) -> Result<Option<Template>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT c.id, c.type_key, c.custom_title, c.custom_body, c.accent_color,                  a.relative_path, c.prayer_key, c.sound_mode, s.relative_path FROM category c LEFT JOIN asset a ON a.sha256 = c.image_id                  LEFT JOIN asset s ON s.sha256 = c.sound_id WHERE c.id = ?1",
                params![id],
                |row| {
                    Ok(Template {
                        id: row.get(0)?,
                        type_key: row.get(1)?,
                        custom_title: row.get(2)?,
                        custom_body: row.get(3)?,
                        accent_color: row.get(4)?,
                        image: row.get(5)?,
                        prayer_key: row.get(6)?,
                        sound_mode: row.get(7)?,
                        sound: row.get(8)?,
                    })
                },
            )
            .optional()?)
    }

    /// The template of a built-in reminder type, by its key.
    /// A prayer's template, seeded by migration 005.
    pub fn template_for_prayer(&self, prayer: &str) -> Result<Option<Template>, StoreError> {
        let id: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM category WHERE prayer_key = ?1",
                params![prayer],
                |row| row.get(0),
            )
            .optional()?;
        id.map_or(Ok(None), |id| self.template(id))
    }

    /// Save what the user wrote for a prayer's template. `None` and empty text
    /// mean "the built-in text in the active language", so "Use Default Text"
    /// is saving `None`. `accent` is `#rrggbb` or `None`; `image` is the
    /// library picture's `sha256` or `None` for the built-in one.
    /// `authored_lang` is the language the text was typed in.
    pub fn set_prayer_template(
        &self,
        prayer: &str,
        edit: &PrayerTemplateEdit,
    ) -> Result<bool, StoreError> {
        let text = |value: &Option<String>| {
            value
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
        };
        let (title, body) = (text(&edit.title), text(&edit.body));
        let lang = (title.is_some() || body.is_some()).then_some(edit.authored_lang);
        let changed = self.conn.execute(
            "UPDATE category SET custom_title = ?2, custom_body = ?3, accent_color = ?4, \
             image_id = ?5, authored_lang = ?6, sound_mode = ?7, sound_id = ?8 \
             WHERE prayer_key = ?1",
            params![
                prayer,
                title,
                body,
                edit.accent,
                edit.image,
                lang,
                edit.sound_mode,
                edit.sound
            ],
        )?;
        Ok(changed == 1)
    }

    /// Record an audio clip that was copied into `assets/`, so a template can
    /// name it. The same clip recorded twice is recorded once.
    pub fn add_audio_asset(&self, clip: &crate::assets::StoredAudio) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO asset (sha256, kind, relative_path, bytes, created_at) \
             VALUES (?1, 'audio', ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params![
                clip.sha256,
                clip.relative_path,
                i64::try_from(clip.bytes).unwrap_or(i64::MAX)
            ],
        )?;
        Ok(())
    }

    /// Point each prayer's managed reminder at its template, so a fire carries
    /// it. Only the link changes: the stored next time is left alone.
    pub fn link_prayer_templates(&self) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE reminder SET category_id = (SELECT c.id FROM category c WHERE c.prayer_key = \
             json_extract(reminder.trigger_json, '$.prayer')) \
             WHERE trigger_type IN ('prayer', 'prayer_before') AND category_id IS NOT \
             (SELECT c.id FROM category c WHERE c.prayer_key = json_extract(reminder.trigger_json, '$.prayer'))",
            [],
        )?;
        Ok(())
    }

    pub fn template_for_type(&self, type_key: &str) -> Result<Option<Template>, StoreError> {
        let id: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM category WHERE type_key = ?1",
                params![type_key],
                |row| row.get(0),
            )
            .optional()?;
        id.map_or(Ok(None), |id| self.template(id))
    }

    /// Record a fire and what became of it. `source` names which prayer source
    /// answered, for Azan; `None` otherwise.
    pub fn log_notification(
        &self,
        reminder_id: Option<i64>,
        fired_at: Timestamp,
        action: NotificationAction,
        source: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO notification_log (reminder_id, fired_at, action, source) \
             VALUES (?1, ?2, ?3, ?4)",
            params![reminder_id, stamp(fired_at), action.as_str(), source],
        )?;
        Ok(())
    }

    /// A reminder's history, oldest first: each fire's time and what became of
    /// it. A row this build cannot read is left out with a log line.
    pub fn history(
        &self,
        reminder_id: i64,
    ) -> Result<Vec<(Timestamp, NotificationAction)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT fired_at, action FROM notification_log WHERE reminder_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map(params![reminder_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut history = Vec::new();
        for row in rows {
            let (fired_at, action) = row?;
            match (fired_at.parse(), NotificationAction::parse(&action)) {
                (Ok(fired_at), Some(action)) => history.push((fired_at, action)),
                _ => tracing::warn!(reminder_id, %fired_at, %action, "a history row was left out"),
            }
        }
        Ok(history)
    }

    /// Delete one reminder's history — Settings ▸ Reminders' Clear History,
    /// after the user confirmed it. Returns how many rows went.
    pub fn clear_history(&self, reminder_id: i64) -> Result<usize, StoreError> {
        Ok(self.conn.execute(
            "DELETE FROM notification_log WHERE reminder_id = ?1",
            params![reminder_id],
        )?)
    }

    /// Drop `notification_log` rows older than 90 days before `now`, at start.
    /// Returns how many went.
    pub fn prune_notification_log(&self, now: Timestamp) -> Result<usize, StoreError> {
        let cutoff = now - SignedDuration::from_hours(NOTIFICATION_LOG_DAYS * 24);
        Ok(self.conn.execute(
            "DELETE FROM notification_log WHERE fired_at < ?1",
            params![stamp(cutoff)],
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> ReminderDraft {
        ReminderDraft {
            category_id: None,
            title: "Bayar bil elektrik".to_owned(),
            body: Some("Sebelum 25hb".to_owned()),
            authored_lang: "ms-MY".to_owned(),
            trigger: Trigger::OffsetBefore {
                at: civil::date(2026, 12, 25).at(9, 0, 0, 0),
                offsets: vec![
                    Offset {
                        amount: 1,
                        unit: OffsetUnit::Weeks,
                    },
                    Offset {
                        amount: 15,
                        unit: OffsetUnit::Minutes,
                    },
                ],
            },
            recurrence: Some(Recurrence {
                every: 2,
                unit: RecurrenceUnit::Weeks,
                weekdays: vec![Weekday::Monday, Weekday::Thursday],
            }),
        }
    }

    fn instant(text: &str) -> Timestamp {
        text.parse().expect("an instant")
    }

    #[test]
    fn a_reminder_round_trips_through_create_read_update_and_delete() {
        let store = Store::open_in_memory().expect("opens");
        let id = store.create_reminder(&draft()).expect("creates");

        let read = store.reminder(id).expect("reads").expect("exists");
        assert_eq!(read.draft, draft(), "stored exactly as written");
        assert_eq!(read.status, Status::Active);
        assert_eq!(
            read.next_fire_at, None,
            "no fire time until the scheduler sets one"
        );

        let mut edited = draft();
        edited.title = "Pay the electricity bill".to_owned();
        edited.authored_lang = "en-US".to_owned();
        edited.trigger = Trigger::At {
            at: civil::date(2027, 2, 28).at(18, 30, 0, 0),
        };
        edited.recurrence = None;
        store
            .set_next_fire_at(id, Some(instant("2026-12-18T01:00:00Z")))
            .expect("the scheduler writes");
        assert!(store.update_reminder(id, &edited).expect("updates"));

        let read = store.reminder(id).expect("reads").expect("exists");
        assert_eq!(read.draft, edited);
        assert_eq!(read.next_fire_at, None, "an edit clears the old fire time");
        assert_eq!(store.reminders().expect("lists").len(), 1);

        assert!(store.delete_reminder(id).expect("deletes"));
        assert_eq!(store.reminder(id).expect("reads"), None);
        assert!(
            !store.delete_reminder(id).expect("a second delete"),
            "nothing left"
        );
        assert!(
            !store
                .update_reminder(id, &edited)
                .expect("update of nothing")
        );
    }

    #[test]
    fn the_trigger_type_column_follows_the_trigger() {
        let store = Store::open_in_memory().expect("opens");
        let id = store.create_reminder(&draft()).expect("creates");
        let kind: String = store
            .connection()
            .query_row(
                "SELECT trigger_type FROM reminder WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .expect("reads");
        assert_eq!(kind, "offset_before");
    }

    #[test]
    fn the_fire_time_is_the_schedulers_and_round_trips() {
        let store = Store::open_in_memory().expect("opens");
        let id = store.create_reminder(&draft()).expect("creates");
        let when = instant("2026-12-18T01:00:00.250Z");
        store.set_next_fire_at(id, Some(when)).expect("writes");
        let read = store.reminder(id).expect("reads").expect("exists");
        assert_eq!(read.next_fire_at, Some(when));

        store.set_next_fire_at(id, None).expect("clears");
        let read = store.reminder(id).expect("reads").expect("exists");
        assert_eq!(read.next_fire_at, None);
    }

    #[test]
    fn a_seeded_template_reads_back_with_its_key_and_no_custom_text() {
        let store = Store::open_in_memory().expect("opens");
        let alarm = store
            .template_for_type("reminder-alarm")
            .expect("reads")
            .expect("seeded");
        assert_eq!(alarm.type_key.as_deref(), Some("reminder-alarm"));
        assert_eq!(alarm.custom_title, None);
        assert_eq!(alarm.custom_body, None);
        assert_eq!(alarm.image, None);
        assert_eq!(store.template(alarm.id).expect("reads"), Some(alarm));
        assert_eq!(store.template(9999).expect("reads"), None);
        assert_eq!(
            store.template_for_type("reminder-nope").expect("reads"),
            None
        );
    }

    #[test]
    fn every_type_in_the_list_has_its_seeded_template() {
        let store = Store::open_in_memory().expect("opens");
        for key in REMINDER_TYPES {
            let template = store.template_for_type(key).expect("reads");
            assert!(template.is_some(), "{key} has no template");
        }
    }

    #[test]
    fn clearing_one_reminders_history_leaves_the_others() {
        let store = Store::open_in_memory().expect("opens");
        let id = store.create_reminder(&draft()).expect("creates");
        let other = store.create_reminder(&draft()).expect("creates");
        let fired = instant("2026-11-19T02:00:00Z");
        for reminder in [id, id, other] {
            store
                .log_notification(Some(reminder), fired, NotificationAction::Shown, None)
                .expect("logs");
        }
        assert_eq!(store.clear_history(id).expect("clears"), 2);
        assert!(store.history(id).expect("reads").is_empty());
        assert_eq!(store.history(other).expect("reads").len(), 1);
    }

    #[test]
    fn history_reads_back_in_order() {
        let store = Store::open_in_memory().expect("opens");
        let id = store.create_reminder(&draft()).expect("creates");
        let fired = instant("2026-11-19T02:00:00Z");
        store
            .log_notification(Some(id), fired, NotificationAction::Shown, None)
            .expect("logs");
        store
            .log_notification(Some(id), fired, NotificationAction::Dismissed, None)
            .expect("logs");
        assert_eq!(
            store.history(id).expect("reads"),
            vec![
                (fired, NotificationAction::Shown),
                (fired, NotificationAction::Dismissed)
            ]
        );
    }

    #[test]
    fn a_fatigue_schedule_round_trips_and_says_what_it_is() {
        let store = Store::open_in_memory().expect("opens");
        for trigger in [
            Trigger::Every { minutes: 20 },
            Trigger::WorkHours { minutes: 480 },
        ] {
            let mut fatigue = draft();
            fatigue.trigger = trigger.clone();
            fatigue.recurrence = None;
            let id = store.create_reminder(&fatigue).expect("creates");
            let read = store.reminder(id).expect("reads").expect("exists");
            assert_eq!(read.draft.trigger, trigger);
            assert!(read.draft.trigger.is_fatigue());
        }
        assert!(!draft().trigger.is_fatigue());
        let kinds: Vec<&str> = FATIGUE.iter().map(|(key, _, _)| *key).collect();
        for key in &kinds {
            assert!(REMINDER_TYPES.contains(key), "{key} is a reminder type");
        }
    }

    #[test]
    fn every_status_round_trips() {
        let store = Store::open_in_memory().expect("opens");
        let id = store.create_reminder(&draft()).expect("creates");
        for status in [
            Status::Done,
            Status::Snoozed,
            Status::Cancelled,
            Status::Active,
        ] {
            store.set_status(id, status).expect("writes");
            let read = store.reminder(id).expect("reads").expect("exists");
            assert_eq!(read.status, status);
        }
    }

    #[test]
    fn a_malformed_trigger_is_refused_not_guessed() {
        let store = Store::open_in_memory().expect("opens");
        let good = store.create_reminder(&draft()).expect("creates");
        // A field this build does not know, a missing field, and not JSON.
        for (id, json) in [
            (
                100,
                r#"{"kind":"at","at":"2026-12-25T09:00:00","repeat":true}"#,
            ),
            (
                101,
                r#"{"kind":"offset_before","at":"2026-12-25T09:00:00"}"#,
            ),
            (102, "not json"),
            (103, r#"{"kind":"whenever"}"#),
        ] {
            store
                .connection()
                .execute(
                    "INSERT INTO reminder (id, title, authored_lang, trigger_type, trigger_json, \
                     created_at, updated_at) VALUES (?1, 'x', 'en-US', 'at', ?2, 'now', 'now')",
                    params![id, json],
                )
                .expect("a raw row");
            let error = store.reminder(id).expect_err(json);
            assert!(
                matches!(
                    error,
                    ReminderError::Malformed {
                        column: "trigger_json",
                        ..
                    }
                ),
                "{json}: {error}"
            );
        }

        // The list leaves the bad rows out and keeps the good one.
        let listed: Vec<i64> = store
            .reminders()
            .expect("lists")
            .iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(listed, vec![good]);
    }

    #[test]
    fn a_recurrence_with_an_unknown_field_is_refused() {
        let store = Store::open_in_memory().expect("opens");
        let id = store.create_reminder(&draft()).expect("creates");
        store
            .connection()
            .execute(
                "UPDATE reminder SET recurrence_json = ?2 WHERE id = ?1",
                params![id, r#"{"every":1,"unit":"days","cron":"* * *"}"#],
            )
            .expect("a raw write");
        let error = store.reminder(id).expect_err("refused");
        assert!(
            matches!(
                error,
                ReminderError::Malformed {
                    column: "recurrence_json",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn authored_lang_accepts_only_the_two_languages() {
        let store = Store::open_in_memory().expect("opens");
        let mut bad = draft();
        bad.authored_lang = "fr-FR".to_owned();
        assert!(store.create_reminder(&bad).is_err());
    }

    #[test]
    fn snoozes_count_up_and_go_with_their_reminder() {
        let store = Store::open_in_memory().expect("opens");
        let id = store.create_reminder(&draft()).expect("creates");
        let other = store.create_reminder(&draft()).expect("creates");

        assert_eq!(store.latest_snooze(id).expect("reads"), None);
        for count in 1..=3 {
            store
                .record_snooze(id, instant("2026-12-18T01:05:00Z"), count)
                .expect("snoozes");
        }
        store
            .record_snooze(other, instant("2026-12-18T01:20:00Z"), 1)
            .expect("snoozes");
        assert_eq!(
            store.latest_snooze(id).expect("reads"),
            Some((instant("2026-12-18T01:05:00Z"), 3)),
            "the latest, with its count"
        );
        assert_eq!(
            store.latest_snooze(other).expect("reads"),
            Some((instant("2026-12-18T01:20:00Z"), 1)),
            "kept per reminder"
        );

        store.delete_reminder(id).expect("deletes");
        let left: i64 = store
            .connection()
            .query_row("SELECT count(*) FROM snooze_log", [], |row| row.get(0))
            .expect("count");
        assert_eq!(
            left, 1,
            "the deleted reminder's snoozes went with it, the other's stayed"
        );
    }

    #[test]
    fn the_notification_log_keeps_89_days_and_drops_91() {
        let store = Store::open_in_memory().expect("opens");
        let id = store.create_reminder(&draft()).expect("creates");
        let now = instant("2026-11-19T00:00:00Z");
        let days_ago = |days: i64| now - SignedDuration::from_hours(days * 24);

        store
            .log_notification(Some(id), days_ago(91), NotificationAction::Shown, None)
            .expect("logs");
        store
            .log_notification(Some(id), days_ago(89), NotificationAction::Dismissed, None)
            .expect("logs");
        store
            .log_notification(None, days_ago(0), NotificationAction::Missed, Some("api"))
            .expect("logs");

        assert_eq!(store.prune_notification_log(now).expect("prunes"), 1);

        let mut stmt = store
            .connection()
            .prepare("SELECT action FROM notification_log ORDER BY fired_at")
            .expect("prepare");
        let actions: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("rows");
        assert_eq!(actions, vec!["dismissed", "missed"]);
    }

    #[test]
    fn a_stamp_is_utc_text_that_sorts_and_parses_back() {
        let when = instant("2026-11-19T08:00:00+08:00");
        assert_eq!(stamp(when), "2026-11-19T00:00:00.000Z");
        assert_eq!(stamp(when).parse::<Timestamp>().expect("parses"), when);
        assert!(stamp(when) < stamp(when + SignedDuration::from_millis(1)));
    }

    #[test]
    fn the_managed_prayer_reminders_follow_what_is_enabled_and_keep_their_rows() {
        let store = Store::open_in_memory().expect("opens");
        let prayers = |store: &Store| -> Vec<(i64, String)> {
            store
                .reminders()
                .expect("reads")
                .into_iter()
                .filter_map(|r| match r.draft.trigger {
                    Trigger::Prayer { prayer } => Some((r.id, prayer)),
                    _ => None,
                })
                .collect()
        };

        assert!(
            store
                .sync_prayer_reminders(&["fajr", "dhuhr"])
                .expect("synced")
        );
        let first = prayers(&store);
        assert_eq!(
            first.iter().map(|(_, p)| p.as_str()).collect::<Vec<_>>(),
            ["fajr", "dhuhr"]
        );

        // The same set again changes nothing and keeps the ids, which carry the
        // stored next time and the history.
        assert!(
            !store
                .sync_prayer_reminders(&["fajr", "dhuhr"])
                .expect("synced")
        );
        assert_eq!(prayers(&store), first);

        // Syuruk on, Dhuhr off.
        assert!(
            store
                .sync_prayer_reminders(&["fajr", "syuruk"])
                .expect("synced")
        );
        let second = prayers(&store);
        assert_eq!(
            second.iter().map(|(_, p)| p.as_str()).collect::<Vec<_>>(),
            ["fajr", "syuruk"]
        );
        assert_eq!(second[0], first[0], "Fajr's row is the same row");

        // A key nobody knows is not made, and an empty set clears them all.
        store
            .sync_prayer_reminders(&["imsak", "fajr"])
            .expect("synced");
        assert_eq!(prayers(&store).len(), 1);
        store.sync_prayer_reminders(&[]).expect("synced");
        assert!(prayers(&store).is_empty());
        assert!(
            Trigger::Prayer {
                prayer: "fajr".into()
            }
            .is_prayer()
        );
        assert!(
            !Trigger::Prayer {
                prayer: "fajr".into()
            }
            .is_fatigue()
        );
    }

    #[test]
    fn every_prayer_has_a_seeded_template_with_nothing_of_the_users_in_it() {
        let store = Store::open_in_memory().expect("opens");
        for prayer in PRAYERS {
            let template = store
                .template_for_prayer(prayer)
                .expect("reads")
                .unwrap_or_else(|| panic!("{prayer} has no template"));
            assert_eq!(template.prayer_key.as_deref(), Some(prayer));
            assert_eq!(
                (
                    template.custom_title,
                    template.custom_body,
                    template.accent_color,
                    template.image
                ),
                (None, None, None, None),
                "the built-in text and picture until the user writes some"
            );
        }
        assert!(store.template_for_prayer("imsak").expect("reads").is_none());
    }

    #[test]
    fn a_prayers_template_keeps_what_was_written_and_default_text_clears_it() {
        let store = Store::open_in_memory().expect("opens");
        let written = PrayerTemplateEdit {
            title: Some("  Masuk Subuh  ".to_owned()),
            body: Some("Bangun dan solat.".to_owned()),
            accent: Some("#3b9eff".to_owned()),
            image: None,
            authored_lang: "ms-MY",
            sound_mode: None,
            sound: None,
        };
        assert!(store.set_prayer_template("fajr", &written).expect("saved"));
        let fajr = store
            .template_for_prayer("fajr")
            .expect("reads")
            .expect("there");
        assert_eq!(fajr.custom_title.as_deref(), Some("Masuk Subuh"), "trimmed");
        assert_eq!(fajr.custom_body.as_deref(), Some("Bangun dan solat."));
        assert_eq!(fajr.accent_color.as_deref(), Some("#3b9eff"));
        // Another prayer is untouched.
        let dhuhr = store
            .template_for_prayer("dhuhr")
            .expect("reads")
            .expect("there");
        assert_eq!(dhuhr.custom_title, None);

        // "Use Default Text": saving nothing restores the built-in.
        let cleared = PrayerTemplateEdit {
            title: Some("   ".to_owned()),
            body: None,
            accent: None,
            image: None,
            authored_lang: "en-US",
            sound_mode: None,
            sound: None,
        };
        store.set_prayer_template("fajr", &cleared).expect("saved");
        let fajr = store
            .template_for_prayer("fajr")
            .expect("reads")
            .expect("there");
        assert_eq!(
            (fajr.custom_title, fajr.custom_body, fajr.accent_color),
            (None, None, None)
        );
        assert!(
            !store
                .set_prayer_template("imsak", &written)
                .expect("no such prayer")
        );
    }

    #[test]
    fn each_prayer_reminder_carries_its_template_after_a_sync() {
        let store = Store::open_in_memory().expect("opens");
        store
            .sync_prayer_reminders(&["fajr", "isha"])
            .expect("synced");
        for reminder in store.reminders().expect("reads") {
            let Trigger::Prayer { prayer } = &reminder.draft.trigger else {
                continue;
            };
            let template = store
                .template_for_prayer(prayer)
                .expect("reads")
                .expect("seeded");
            assert_eq!(reminder.draft.category_id, Some(template.id), "{prayer}");
        }
        // The link is repaired, not only made: a row with none gets it back,
        // and its stored next time is left alone.
        let id = store.reminders().expect("reads")[0].id;
        store
            .set_next_fire_at(id, Some("2026-11-19T05:00:00Z".parse().expect("instant")))
            .expect("stored");
        store
            .connection()
            .execute("UPDATE reminder SET category_id = NULL", [])
            .expect("unlinked");
        store
            .sync_prayer_reminders(&["fajr", "isha"])
            .expect("synced");
        let again = store.reminder(id).expect("reads").expect("there");
        assert!(again.draft.category_id.is_some(), "the link is back");
        assert_eq!(
            again.next_fire_at,
            Some("2026-11-19T05:00:00Z".parse().expect("instant"))
        );
    }
    #[test]
    fn a_prayers_sound_is_the_chime_until_chosen_and_an_imported_clip_is_named_by_its_asset() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let store = Store::open_in_memory().expect("opens");
        let fajr = store
            .template_for_prayer("fajr")
            .expect("reads")
            .expect("there");
        assert_eq!(
            (fajr.sound_mode.as_deref(), fajr.sound.as_deref()),
            (None, None),
            "the chime"
        );

        // A clip, copied in under its hash and recorded; storing it twice is one file.
        let clip =
            crate::assets::store_audio(dir.path(), b"RIFFxxxxWAVEdata", "wav").expect("stored");
        assert_eq!(clip.relative_path, format!("assets/{}.wav", clip.sha256));
        assert!(
            dir.path().join(&clip.relative_path).is_file(),
            "it landed where the path says"
        );
        assert_eq!(
            crate::assets::store_audio(dir.path(), b"RIFFxxxxWAVEdata", "wav").expect("again"),
            clip
        );
        store.add_audio_asset(&clip).expect("recorded");
        store.add_audio_asset(&clip).expect("recorded again");

        let edit = |mode: &str, sound: Option<String>| PrayerTemplateEdit {
            title: None,
            body: None,
            accent: None,
            image: None,
            authored_lang: "en-US",
            sound_mode: Some(mode.to_owned()),
            sound,
        };
        store
            .set_prayer_template("fajr", &edit("file", Some(clip.sha256.clone())))
            .expect("saved");
        let fajr = store
            .template_for_prayer("fajr")
            .expect("reads")
            .expect("there");
        assert_eq!(fajr.sound_mode.as_deref(), Some("file"));
        assert_eq!(fajr.sound.as_deref(), Some(clip.relative_path.as_str()));

        store
            .set_prayer_template("fajr", &edit("none", None))
            .expect("saved");
        assert_eq!(
            store
                .template_for_prayer("fajr")
                .expect("reads")
                .expect("there")
                .sound_mode
                .as_deref(),
            Some("none")
        );
        // A mode nobody knows is refused by the table, not stored.
        assert!(
            store
                .set_prayer_template("fajr", &edit("loud", None))
                .is_err()
        );
    }
}
