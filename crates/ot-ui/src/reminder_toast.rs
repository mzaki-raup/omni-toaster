//! What a reminder's toast says: the one function that turns a reminder and
//! its template into toast content (`.claude/rules/ui.md`, "A toast is drawn
//! only from a `ToastSpec`"). The scheduler's fires and, from FEAT-0046, the
//! reminder editor's preview both come through here, and both hand the result
//! to `ot_toast::Prefs::spec`, so the preview and the real toast cannot
//! disagree.
//!
//! The user's own words win and are never translated; the template's custom
//! text comes next; the built-in text of the type, in the active language,
//! comes last (`docs/specs/toast.md` §8, `docs/10-i18n.md`).

use ot_store::reminders::{Reminder, Template};

/// The type a reminder with no template, or a template of no type, counts as.
pub const DEFAULT_TYPE: &str = "reminder-custom";

/// The type that goes to the front of the toast queue
/// (`docs/specs/reminders.md` §8). Azan joins it in Phase 5.
const URGENT_TYPE: &str = "reminder-alarm";

/// The toast's content for a reminder. `timestamp` is already formatted, by
/// the one formatter, as everything in a spec is text.
pub fn content(
    reminder: &Reminder,
    template: Option<&Template>,
    timestamp: Option<String>,
) -> ot_toast::Content {
    let type_key = type_key(template);
    let title = non_empty(Some(reminder.draft.title.as_str()))
        .or_else(|| non_empty(template.and_then(|t| t.custom_title.as_deref())))
        .map_or_else(|| ot_i18n::text(type_key), str::to_owned);
    let body = non_empty(reminder.draft.body.as_deref())
        .or_else(|| non_empty(template.and_then(|t| t.custom_body.as_deref())))
        .map_or_else(|| ot_i18n::text(&format!("{type_key}-body")), str::to_owned);
    let image = template
        .and_then(|t| t.image.clone())
        .or_else(|| default_image(type_key).map(str::to_owned));

    ot_toast::Content {
        title,
        body,
        app_name: ot_i18n::text("toast-app-name"),
        image,
        accent: template
            .and_then(|t| t.accent_color.as_deref())
            .and_then(parse_accent),
        timestamp,
        action_labels: action_labels(),
        ..ot_toast::Content::default()
    }
}

/// Whether a reminder of this template jumps the toast queue.
pub fn is_urgent(template: Option<&Template>) -> bool {
    type_key(template) == URGENT_TYPE
}

/// The action row's labels in the active language, for every toast.
pub fn action_labels() -> ot_toast::ActionLabels {
    ot_toast::ActionLabels {
        open: ot_i18n::text("toast-action-open"),
        snooze: ot_i18n::text("toast-action-snooze"),
        dismiss: ot_i18n::text("toast-action-dismiss"),
    }
}

fn type_key(template: Option<&Template>) -> &str {
    template
        .and_then(|t| t.type_key.as_deref())
        .filter(|key| ot_store::reminders::REMINDER_TYPES.contains(key))
        .unwrap_or(DEFAULT_TYPE)
}

/// The built-in picture a type shows until its template chooses one
/// (`assets/README.md` records which picture is for which type).
fn default_image(type_key: &str) -> Option<&'static str> {
    match type_key {
        "reminder-alarm" | "reminder-timer" => Some("@builtin/alarm-clock"),
        "reminder-appointment" => Some("@builtin/meeting"),
        "reminder-event" => Some("@builtin/training"),
        _ => None,
    }
}

fn non_empty(text: Option<&str>) -> Option<&str> {
    text.filter(|t| !t.trim().is_empty())
}

/// `#rrggbb`, as the template stores an accent. Anything else is no accent,
/// so the global one applies.
fn parse_accent(text: &str) -> Option<ot_toast::Rgba> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).ok();
    Some(ot_toast::Rgba::new(
        channel(0)?,
        channel(2)?,
        channel(4)?,
        0xff,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ot_i18n::Language;
    use ot_store::reminders::{ReminderDraft, Status, Trigger};

    /// The process-wide bundle, in `language`: tolerates a sibling test
    /// having initialised it first, and asserts the switch took
    /// (docs/GOTCHAS.md, Testing).
    fn in_language(language: Language) {
        let _ = ot_i18n::init(language);
        ot_i18n::set_language(language).expect("switches");
        assert_eq!(ot_i18n::current_language(), Some(language));
    }

    /// The i18n bundle is global, so the tests that switch it take turns.
    static LANGUAGE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn reminder(title: &str, body: Option<&str>) -> Reminder {
        Reminder {
            id: 7,
            draft: ReminderDraft {
                category_id: Some(9),
                title: title.to_owned(),
                body: body.map(str::to_owned),
                authored_lang: "ms-MY".to_owned(),
                trigger: Trigger::At {
                    at: "2026-11-19T10:00".parse().expect("civil"),
                },
                recurrence: None,
            },
            next_fire_at: None,
            status: Status::Active,
        }
    }

    fn template(type_key: &str) -> Template {
        Template {
            id: 9,
            type_key: Some(type_key.to_owned()),
            ..Template::default()
        }
    }

    #[test]
    fn every_type_has_a_name_and_a_body_in_both_catalogues() {
        for tag in ["en-US", "ms-MY"] {
            let keys = ot_i18n::message_keys(tag).expect("parses");
            for key in ot_store::reminders::REMINDER_TYPES {
                assert!(keys.contains(*key), "{key} missing from {tag}");
                let body = format!("{key}-body");
                assert!(keys.contains(&body), "{body} missing from {tag}");
            }
        }
    }

    #[test]
    fn the_users_title_and_body_are_untouched_in_both_languages() {
        let _turn = LANGUAGE.lock().unwrap_or_else(|e| e.into_inner());
        let typed = reminder("Bayar bil TNB", Some("Sebelum 25hb"));
        for language in [Language::English, Language::Malay] {
            in_language(language);
            let content = content(&typed, Some(&template("reminder-payment")), None);
            assert_eq!(content.title, "Bayar bil TNB");
            assert_eq!(content.body, "Sebelum 25hb");
        }
    }

    #[test]
    fn the_template_then_the_type_fill_what_the_user_left_empty() {
        let _turn = LANGUAGE.lock().unwrap_or_else(|e| e.into_inner());
        in_language(Language::Malay);
        let mut custom = template("reminder-alarm");
        custom.custom_body = Some("Bangun!".to_owned());
        let from_template = content(&reminder("Subuh", None), Some(&custom), None);
        assert_eq!(from_template.body, "Bangun!");

        let built_in = content(
            &reminder(" ", None),
            Some(&template("reminder-alarm")),
            None,
        );
        assert_eq!(built_in.title, "Penggera", "the type's name, in Malay");
        assert_eq!(built_in.body, "Penggera anda sedang berbunyi.");

        in_language(Language::English);
        let no_template = content(&reminder("", None), None, None);
        assert_eq!(no_template.title, "Custom");
        assert_eq!(no_template.body, "This is your reminder.");
    }

    #[test]
    fn pictures_and_accent_come_from_the_template_or_the_type() {
        let alarm = content(
            &reminder("x", None),
            Some(&template("reminder-alarm")),
            None,
        );
        assert_eq!(alarm.image.as_deref(), Some("@builtin/alarm-clock"));

        let mut chosen = template("reminder-alarm");
        chosen.image = Some("assets/ab.png".to_owned());
        chosen.accent_color = Some("#1a2b3c".to_owned());
        let custom = content(&reminder("x", None), Some(&chosen), None);
        assert_eq!(custom.image.as_deref(), Some("assets/ab.png"));
        assert_eq!(
            custom.accent,
            Some(ot_toast::Rgba::new(0x1a, 0x2b, 0x3c, 0xff))
        );

        let task = content(&reminder("x", None), Some(&template("reminder-task")), None);
        assert_eq!(task.image, None);
        assert_eq!(task.accent, None);
        assert_eq!(parse_accent("1a2b3c"), None);
        assert_eq!(parse_accent("#1a2b3"), None);
        assert_eq!(parse_accent("#zz2b3c"), None);
    }

    #[test]
    fn only_an_alarm_is_urgent() {
        assert!(is_urgent(Some(&template("reminder-alarm"))));
        assert!(!is_urgent(Some(&template("reminder-timer"))));
        assert!(!is_urgent(None));
    }

    #[test]
    fn a_preview_and_a_fire_build_the_same_toast() {
        // Both callers go through `content` and `Prefs::spec`; the same
        // inputs must give the same spec, field for field.
        let typed = reminder("Mesyuarat", None);
        let prefs = ot_toast::Prefs::default();
        let tpl = template("reminder-appointment");
        let preview = prefs.spec(content(&typed, Some(&tpl), Some("10:00".to_owned())));
        let fired = prefs.spec(content(&typed, Some(&tpl), Some("10:00".to_owned())));
        assert_eq!(preview, fired);
    }
}
