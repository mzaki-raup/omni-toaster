//! The reminder editor in Settings ▸ Reminders (`docs/specs/reminders.md`
//! §4, §10).
//!
//! [`Editor`] is what the user is editing, [`Editor::validate`] says what is
//! wrong with it in the user's language and against which field, and
//! [`editor`] draws it. Validation is a pure function of the editor, the
//! instant and the zone, so it is tested without a window.

use jiff::Timestamp;
use jiff::civil::{Date, DateTime, Time};
use jiff::tz::TimeZone;
use ot_store::reminders::{
    Offset, OffsetUnit, Recurrence, RecurrenceUnit, Reminder, ReminderDraft, Trigger, Weekday,
};

use crate::layout::{self, Columns};
use crate::theme::Tokens;

/// The most an offset's amount may be: 999 minutes, hours, days or weeks.
const OFFSET_MAX: u32 = 999;
/// The most a repeat's interval may be: every 999 days, weeks and so on.
const EVERY_MAX: u32 = 999;
/// A text field's own margin, left and right, which egui adds to its desired
/// width: without taking it off, the field runs past the dropdowns' edge.
const TEXT_MARGIN: f32 = 4.0;

/// What is being edited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editor {
    /// `None` for a reminder not saved yet.
    pub id: Option<i64>,
    pub title: String,
    pub body: String,
    /// A key of `ot_store::reminders::REMINDER_TYPES`.
    pub type_key: &'static str,
    pub date: Date,
    pub time: Time,
    pub offsets: Vec<Offset>,
    /// `None` is "Never": it fires once.
    pub repeat: Option<RecurrenceUnit>,
    pub every: u32,
    pub weekdays: Vec<Weekday>,
    /// Set while the Delete confirmation is up.
    pub confirming_delete: bool,
    /// "Repeats every 2 weeks on Monday and Thursday.", and what it was built
    /// from, so it is formatted when the repeat changes, not every frame.
    pub summary: Option<(SummaryKey, String)>,
}

/// What the repeat summary depends on.
pub type SummaryKey = (
    Option<RecurrenceUnit>,
    u32,
    Vec<Weekday>,
    Option<ot_i18n::Language>,
);

/// Which field a problem is about, so it is said under that field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Title,
    When,
}

/// What the editor's buttons asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorAction {
    Save,
    Cancel,
    /// Delete, after the confirmation.
    Delete,
}

impl Editor {
    /// A new reminder at the next whole hour, of the Custom type.
    pub fn new_at(now: Timestamp, tz: &TimeZone) -> Self {
        let local = now.to_zoned_datetime(tz.clone()).datetime();
        let next_hour = local
            .checked_add(jiff::SignedDuration::from_hours(1))
            .unwrap_or(local);
        Self {
            id: None,
            title: String::new(),
            body: String::new(),
            type_key: crate::reminder_toast::DEFAULT_TYPE,
            date: next_hour.date(),
            time: Time::new(next_hour.hour(), 0, 0, 0).unwrap_or_default(),
            offsets: Vec::new(),
            repeat: None,
            every: 1,
            weekdays: Vec::new(),
            confirming_delete: false,
            summary: None,
        }
    }

    /// Edit a stored reminder. `type_key` is its template's type, if any.
    pub fn from_reminder(reminder: &Reminder, type_key: Option<&'static str>) -> Self {
        let (at, offsets) = match &reminder.draft.trigger {
            Trigger::At { at } => (*at, Vec::new()),
            Trigger::OffsetBefore { at, offsets } => (*at, offsets.clone()),
        };
        let (repeat, every, weekdays) = match &reminder.draft.recurrence {
            Some(recurrence) => (
                Some(recurrence.unit),
                recurrence.every.max(1),
                recurrence.weekdays.clone(),
            ),
            None => (None, 1, Vec::new()),
        };
        Self {
            id: Some(reminder.id),
            title: reminder.draft.title.clone(),
            body: reminder.draft.body.clone().unwrap_or_default(),
            type_key: type_key.unwrap_or(crate::reminder_toast::DEFAULT_TYPE),
            date: at.date(),
            time: Time::new(at.hour(), at.minute(), 0, 0).unwrap_or_default(),
            offsets,
            repeat,
            every,
            weekdays,
            confirming_delete: false,
            summary: None,
        }
    }

    /// The reminder's local date and time.
    pub fn at(&self) -> DateTime {
        self.date.to_datetime(self.time)
    }

    /// What is stored. `category_id` is the type's template; `authored_lang`
    /// the language the title was typed in, which is the UI's.
    pub fn draft(&self, category_id: Option<i64>, authored_lang: &str) -> ReminderDraft {
        let at = self.at();
        let trigger = if self.offsets.is_empty() {
            Trigger::At { at }
        } else {
            Trigger::OffsetBefore {
                at,
                offsets: self.offsets.clone(),
            }
        };
        let recurrence = self.repeat.map(|unit| Recurrence {
            every: self.every.clamp(1, EVERY_MAX),
            unit,
            weekdays: if unit == RecurrenceUnit::Weeks {
                sorted_weekdays(&self.weekdays)
            } else {
                Vec::new()
            },
        });
        let body = self.body.trim();
        ReminderDraft {
            category_id,
            // Stored as typed: only the surrounding blanks go.
            title: self.title.trim().to_owned(),
            body: (!body.is_empty()).then(|| body.to_owned()),
            authored_lang: authored_lang.to_owned(),
            trigger,
            recurrence,
        }
    }

    /// What is wrong, as Fluent keys against their fields. Empty when it can
    /// be saved.
    pub fn validate(&self, now: Timestamp, tz: &TimeZone) -> Vec<(Field, &'static str)> {
        let mut problems = Vec::new();
        if self.title.trim().is_empty() {
            problems.push((Field::Title, "error-reminder-title-empty"));
        }
        // A reminder that would never fire again is not saved: a one-off in
        // the past, or a repeat whose every time is behind it.
        let draft = self.draft(None, "en-US");
        if ot_sched::recurrence::next_fire(&draft.trigger, draft.recurrence.as_ref(), now, tz)
            .is_none()
        {
            problems.push((Field::When, "error-reminder-time-passed"));
        }
        problems
    }
}

/// The weekdays Monday first, once each.
fn sorted_weekdays(days: &[Weekday]) -> Vec<Weekday> {
    ALL_WEEKDAYS
        .iter()
        .copied()
        .filter(|day| days.contains(day))
        .collect()
}

const ALL_WEEKDAYS: [Weekday; 7] = [
    Weekday::Monday,
    Weekday::Tuesday,
    Weekday::Wednesday,
    Weekday::Thursday,
    Weekday::Friday,
    Weekday::Saturday,
    Weekday::Sunday,
];

fn weekday_name(day: Weekday) -> String {
    ot_i18n::format::weekday_name(match day {
        Weekday::Monday => jiff::civil::Weekday::Monday,
        Weekday::Tuesday => jiff::civil::Weekday::Tuesday,
        Weekday::Wednesday => jiff::civil::Weekday::Wednesday,
        Weekday::Thursday => jiff::civil::Weekday::Thursday,
        Weekday::Friday => jiff::civil::Weekday::Friday,
        Weekday::Saturday => jiff::civil::Weekday::Saturday,
        Weekday::Sunday => jiff::civil::Weekday::Sunday,
    })
}

fn offset_unit_key(unit: OffsetUnit) -> &'static str {
    match unit {
        OffsetUnit::Minutes => "settings-unit-minutes",
        OffsetUnit::Hours => "settings-unit-hours",
        OffsetUnit::Days => "settings-unit-days",
        OffsetUnit::Weeks => "settings-unit-weeks",
    }
}

fn repeat_key(repeat: Option<RecurrenceUnit>) -> &'static str {
    match repeat {
        None => "settings-repeat-never",
        Some(RecurrenceUnit::Days) => "settings-unit-days",
        Some(RecurrenceUnit::Weeks) => "settings-unit-weeks",
        Some(RecurrenceUnit::Months) => "settings-unit-months",
        Some(RecurrenceUnit::Years) => "settings-unit-years",
    }
}

/// "Repeats every 2 weeks on Monday and Thursday.", rebuilt only when the
/// repeat or the language changed.
fn repeat_summary(editor: &mut Editor) -> Option<String> {
    let unit = editor.repeat?;
    let key: SummaryKey = (
        Some(unit),
        editor.every,
        sorted_weekdays(&editor.weekdays),
        ot_i18n::current_language(),
    );
    if editor.summary.as_ref().map(|(k, _)| k) != Some(&key) {
        let recurrence = Recurrence {
            every: editor.every.clamp(1, EVERY_MAX),
            unit,
            weekdays: if unit == RecurrenceUnit::Weeks {
                key.2.clone()
            } else {
                Vec::new()
            },
        };
        let text = ot_i18n::text_with(
            "settings-reminder-repeat-summary",
            &[("repeat", crate::phrases::repeat(&recurrence).into())],
        );
        editor.summary = Some((key, text));
    }
    editor.summary.as_ref().map(|(_, text)| text.clone())
}

/// Draw the editor. `problems` comes from [`Editor::validate`]; each is said
/// under its field. Returns what a button asked for.
pub fn editor(
    ui: &mut egui::Ui,
    columns: Columns,
    editor: &mut Editor,
    problems: &[(Field, &'static str)],
    tokens: Tokens,
) -> Option<EditorAction> {
    let mut action = None;
    let problem = |field| {
        problems
            .iter()
            .find(|(f, _)| *f == field)
            .map(|(_, key)| *key)
    };
    let say = |ui: &mut egui::Ui, field| {
        if let Some(key) = problem(field) {
            layout::field_hint(ui, columns, &ot_i18n::text(key), tokens.danger);
        }
    };

    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("settings-reminder-title"),
        |ui| {
            let height = layout::field_height(ui);
            ui.add(
                egui::TextEdit::singleline(&mut editor.title)
                    .margin(egui::Margin::symmetric(TEXT_MARGIN as i8, 2))
                    .desired_width(columns.field - 2.0 * TEXT_MARGIN)
                    .min_size(egui::vec2(0.0, height))
                    .vertical_align(egui::Align::Center),
            );
        },
    );
    say(ui, Field::Title);
    ui.add_space(layout::ROW_GAP);

    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("settings-reminder-body"),
        |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut editor.body)
                    .margin(egui::Margin::symmetric(TEXT_MARGIN as i8, 2))
                    .desired_width(columns.field - 2.0 * TEXT_MARGIN)
                    .desired_rows(2),
            );
        },
    );
    ui.add_space(layout::ROW_GAP);

    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("settings-reminder-type"),
        |ui| {
            egui::ComboBox::from_id_salt("reminder-type")
                .width(columns.field)
                .height(layout::field_height(ui))
                .selected_text(ot_i18n::text(editor.type_key))
                .show_ui(ui, |ui| {
                    for key in ot_store::reminders::REMINDER_TYPES {
                        ui.selectable_value(&mut editor.type_key, key, ot_i18n::text(key));
                    }
                });
        },
    );
    ui.add_space(layout::ROW_GAP);

    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("settings-reminder-date"),
        |ui| {
            crate::fields::date_field(ui, "reminder-date", &mut editor.date);
        },
    );
    ui.add_space(layout::ROW_GAP);
    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("settings-reminder-time"),
        |ui| {
            crate::fields::time_field(ui, &mut editor.time);
        },
    );
    say(ui, Field::When);
    ui.add_space(layout::ROW_GAP);

    // Offsets: each a row of amount, unit and Remove, then Add.
    let mut remove = None;
    for (index, offset) in editor.offsets.iter_mut().enumerate() {
        let label = if index == 0 {
            ot_i18n::text("settings-reminder-offsets")
        } else {
            String::new()
        };
        layout::labelled_row(ui, columns, &label, |ui| {
            ui.horizontal_wrapped(|ui| {
                let height = layout::field_height(ui);
                ui.add(egui::DragValue::new(&mut offset.amount).range(1..=OFFSET_MAX));
                egui::ComboBox::from_id_salt(("offset-unit", index))
                    .height(height)
                    .selected_text(ot_i18n::text(offset_unit_key(offset.unit)))
                    .show_ui(ui, |ui| {
                        for unit in [
                            OffsetUnit::Minutes,
                            OffsetUnit::Hours,
                            OffsetUnit::Days,
                            OffsetUnit::Weeks,
                        ] {
                            ui.selectable_value(
                                &mut offset.unit,
                                unit,
                                ot_i18n::text(offset_unit_key(unit)),
                            );
                        }
                    });
                if ui
                    .add(
                        egui::Button::new(ot_i18n::text("settings-reminder-offset-remove"))
                            .min_size(egui::vec2(0.0, height)),
                    )
                    .clicked()
                {
                    remove = Some(index);
                }
            });
        });
    }
    if let Some(index) = remove {
        editor.offsets.remove(index);
    }
    let add_label = if editor.offsets.is_empty() {
        ot_i18n::text("settings-reminder-offsets")
    } else {
        String::new()
    };
    layout::labelled_row(ui, columns, &add_label, |ui| {
        let height = layout::field_height(ui);
        if ui
            .add(
                egui::Button::new(ot_i18n::text("settings-reminder-offset-add"))
                    .min_size(egui::vec2(0.0, height)),
            )
            .clicked()
        {
            editor.offsets.push(Offset {
                amount: 15,
                unit: OffsetUnit::Minutes,
            });
        }
    });
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("settings-reminder-offsets-hint"),
        tokens.text_dim,
    );
    ui.add_space(layout::ROW_GAP);

    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("settings-reminder-repeat"),
        |ui| {
            egui::ComboBox::from_id_salt("reminder-repeat")
                .width(columns.field)
                .height(layout::field_height(ui))
                .selected_text(ot_i18n::text(repeat_key(editor.repeat)))
                .show_ui(ui, |ui| {
                    for repeat in [
                        None,
                        Some(RecurrenceUnit::Days),
                        Some(RecurrenceUnit::Weeks),
                        Some(RecurrenceUnit::Months),
                        Some(RecurrenceUnit::Years),
                    ] {
                        ui.selectable_value(
                            &mut editor.repeat,
                            repeat,
                            ot_i18n::text(repeat_key(repeat)),
                        );
                    }
                });
        },
    );
    if let Some(summary) = repeat_summary(editor) {
        layout::field_hint(ui, columns, &summary, tokens.text_dim);
    }
    if editor.repeat.is_some() {
        ui.add_space(layout::ROW_GAP);
        layout::labelled_row(
            ui,
            columns,
            &ot_i18n::text("settings-reminder-every"),
            |ui| {
                ui.add(egui::DragValue::new(&mut editor.every).range(1..=EVERY_MAX));
            },
        );
    }
    if editor.repeat == Some(RecurrenceUnit::Weeks) {
        ui.add_space(layout::ROW_GAP);
        layout::labelled_row(
            ui,
            columns,
            &ot_i18n::text("settings-reminder-weekdays"),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    for day in ALL_WEEKDAYS {
                        let mut on = editor.weekdays.contains(&day);
                        if ui.checkbox(&mut on, weekday_name(day)).changed() {
                            if on {
                                editor.weekdays.push(day);
                            } else {
                                editor.weekdays.retain(|d| *d != day);
                            }
                        }
                    }
                });
            },
        );
        layout::field_hint(
            ui,
            columns,
            &ot_i18n::text("settings-reminder-weekdays-hint"),
            tokens.text_dim,
        );
    }
    ui.add_space(layout::ROW_GAP);

    // The buttons, or the Delete confirmation in their place.
    ui.horizontal_wrapped(|ui| {
        ui.add_space(columns.label);
        let height = layout::field_height(ui);
        let button = |text: &str| egui::Button::new(text).min_size(egui::vec2(0.0, height));
        if editor.confirming_delete {
            ui.label(ot_i18n::text("settings-reminder-delete-confirm"));
            if ui
                .add(button(&ot_i18n::text("settings-reminder-delete")))
                .clicked()
            {
                action = Some(EditorAction::Delete);
            }
            if ui
                .add(button(&ot_i18n::text("settings-reminder-keep")))
                .clicked()
            {
                editor.confirming_delete = false;
            }
            return;
        }
        if ui
            .add_enabled(
                problems.is_empty(),
                button(&ot_i18n::text("settings-reminder-save")),
            )
            .clicked()
        {
            action = Some(EditorAction::Save);
        }
        if ui
            .add(button(&ot_i18n::text("settings-reminder-cancel")))
            .clicked()
        {
            action = Some(EditorAction::Cancel);
        }
        if editor.id.is_some()
            && ui
                .add(button(&ot_i18n::text("settings-reminder-delete")))
                .clicked()
        {
            editor.confirming_delete = true;
        }
    });
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant(text: &str) -> Timestamp {
        text.parse().expect("an instant")
    }

    fn editor_at(at: &str) -> Editor {
        let mut editor = Editor::new_at(instant("2026-11-19T00:00Z"), &TimeZone::UTC);
        let at: DateTime = at.parse().expect("civil");
        editor.date = at.date();
        editor.time = at.time();
        editor.title = "Bayar bil".to_owned();
        editor
    }

    #[test]
    fn a_new_reminder_starts_at_the_next_whole_hour_as_custom() {
        let editor = Editor::new_at(instant("2026-11-19T09:41Z"), &TimeZone::UTC);
        assert_eq!(
            editor.at(),
            "2026-11-19T10:00".parse::<DateTime>().expect("civil")
        );
        assert_eq!(editor.type_key, "reminder-custom");
        assert!(editor.id.is_none());
    }

    #[test]
    fn an_empty_title_and_a_past_one_off_are_said_against_their_fields() {
        let now = instant("2026-11-19T12:00Z");
        let mut editor = editor_at("2026-11-19T10:00");
        editor.title = "   ".to_owned();
        assert_eq!(
            editor.validate(now, &TimeZone::UTC),
            vec![
                (Field::Title, "error-reminder-title-empty"),
                (Field::When, "error-reminder-time-passed"),
            ]
        );

        // Repeating, the same time is fine: it comes round again.
        editor.title = "Minum air".to_owned();
        editor.repeat = Some(RecurrenceUnit::Days);
        assert!(editor.validate(now, &TimeZone::UTC).is_empty());
    }

    #[test]
    fn a_passed_time_with_offsets_is_refused_and_a_time_ahead_is_not() {
        // The time itself passed and the offsets are before it, so every
        // fire is behind: refused.
        let now = instant("2026-11-19T12:00Z");
        let mut editor = editor_at("2026-11-19T10:00");
        editor.offsets.push(Offset {
            amount: 1,
            unit: OffsetUnit::Hours,
        });
        assert_eq!(
            editor.validate(now, &TimeZone::UTC),
            vec![(Field::When, "error-reminder-time-passed")]
        );
        // The time still ahead with an offset already behind: valid, the
        // time itself still fires.
        let mut ahead = editor_at("2026-11-19T12:30");
        ahead.offsets.push(Offset {
            amount: 1,
            unit: OffsetUnit::Hours,
        });
        assert!(ahead.validate(now, &TimeZone::UTC).is_empty());
    }

    #[test]
    fn the_draft_round_trips_through_a_stored_reminder() {
        let mut editor = editor_at("2026-12-25T09:30");
        editor.body = "  Sebelum 25hb  ".to_owned();
        editor.type_key = "reminder-payment";
        editor.offsets = vec![Offset {
            amount: 2,
            unit: OffsetUnit::Days,
        }];
        editor.repeat = Some(RecurrenceUnit::Weeks);
        editor.every = 2;
        editor.weekdays = vec![Weekday::Thursday, Weekday::Monday];

        let draft = editor.draft(Some(10), "ms-MY");
        assert_eq!(draft.body.as_deref(), Some("Sebelum 25hb"));
        assert_eq!(draft.authored_lang, "ms-MY");
        assert_eq!(
            draft.recurrence.as_ref().map(|r| r.weekdays.clone()),
            Some(vec![Weekday::Monday, Weekday::Thursday]),
            "Monday first, whatever order they were ticked in"
        );

        let stored = Reminder {
            id: 4,
            draft: draft.clone(),
            next_fire_at: None,
            status: ot_store::reminders::Status::Active,
        };
        let back = Editor::from_reminder(&stored, Some("reminder-payment"));
        assert_eq!(back.draft(Some(10), "ms-MY"), draft);
        assert_eq!(back.id, Some(4));
    }

    #[test]
    fn weekdays_are_kept_only_for_a_weekly_repeat_and_an_empty_body_is_none() {
        let mut editor = editor_at("2026-12-25T09:30");
        editor.repeat = Some(RecurrenceUnit::Months);
        editor.weekdays = vec![Weekday::Monday];
        let draft = editor.draft(None, "en-US");
        assert_eq!(draft.recurrence.map(|r| r.weekdays), Some(Vec::new()));
        assert_eq!(draft.body, None);
        assert!(
            matches!(draft.trigger, Trigger::At { .. }),
            "no offsets, no OffsetBefore"
        );
    }
}
