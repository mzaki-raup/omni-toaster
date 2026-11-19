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
}

impl Trigger {
    /// The `trigger_type` column's value.
    fn type_name(&self) -> &'static str {
        match self {
            Self::At { .. } => "at",
            Self::OffsetBefore { .. } => "offset_before",
        }
    }
}

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

    /// Record a snooze and return its running count, starting at 1.
    pub fn record_snooze(&self, reminder_id: i64, until: Timestamp) -> Result<u32, StoreError> {
        let count = self.snooze_count(reminder_id)? + 1;
        self.conn.execute(
            "INSERT INTO snooze_log (reminder_id, snoozed_until, count) VALUES (?1, ?2, ?3)",
            params![reminder_id, stamp(until), count],
        )?;
        Ok(count)
    }

    /// How many times a reminder has been snoozed.
    pub fn snooze_count(&self, reminder_id: i64) -> Result<u32, StoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT coalesce(max(count), 0) FROM snooze_log WHERE reminder_id = ?1",
            params![reminder_id],
            |row| row.get(0),
        )?;
        Ok(u32::try_from(count).unwrap_or(u32::MAX))
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

        assert_eq!(store.snooze_count(id).expect("count"), 0);
        for expected in 1..=3 {
            let count = store
                .record_snooze(id, instant("2026-12-18T01:05:00Z"))
                .expect("snoozes");
            assert_eq!(count, expected);
        }
        store
            .record_snooze(other, instant("2026-12-18T01:05:00Z"))
            .expect("snoozes");
        assert_eq!(
            store.snooze_count(other).expect("count"),
            1,
            "counted per reminder"
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
}
