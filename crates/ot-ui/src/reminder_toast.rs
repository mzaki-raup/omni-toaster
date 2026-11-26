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

/// A fire's whole toast: [`content`], the user's style, and Snooze — which
/// works on a reminder's toast, until the fire has been snoozed `limit`
/// times, when the row offers only Dismiss and says why
/// (`docs/specs/reminders.md` §5). `before` is an offset fire's phrase, "1 day
/// before", shown in the action row unless the limit's line takes it. An
/// urgent type's toast is persistent: it
/// stays until acted on, up to the ten-minute cap, rather than for the
/// display duration (`docs/specs/toast.md` §6).
pub fn spec(
    prefs: &ot_toast::Prefs,
    reminder: &Reminder,
    template: Option<&Template>,
    timestamp: Option<String>,
    before: Option<String>,
    snoozes: u32,
    limit: u32,
) -> ot_toast::ToastSpec {
    let mut content = content(reminder, template, timestamp);
    content.snoozable = true;
    let at_limit = snoozes >= limit;
    content.note = if at_limit {
        Some(ot_i18n::text("toast-snooze-limit"))
    } else {
        before
    };
    let mut spec = prefs.spec(content);
    if at_limit {
        spec.behaviour.actions = 1;
    }
    if is_urgent(template) {
        spec.behaviour.persistent = true;
        spec.behaviour.duration_secs = ot_toast::spec::PERSISTENT_CAP_SECS;
    }
    spec
}

/// A prayer's toast: the template's own title and message if it has them,
/// else the prayer's name with both names and "It is time for …"; its own
/// picture and accent, else the built-in masjid picture and the global accent;
/// the day's source label as the note. Dismiss only, persistent up to the
/// ten-minute cap, drawn from the same `ToastSpec` as every other toast
/// (`docs/11-prayer-and-azan.md` §9). The fire and the editor's preview both
/// come through here, so they cannot disagree. `prayer` is the scheduler's key.
pub fn prayer_spec(
    prefs: &ot_toast::Prefs,
    prayer: &str,
    template: Option<&Template>,
    timestamp: Option<String>,
    note: Option<String>,
) -> ot_toast::ToastSpec {
    let name = crate::prayer_pane::prayer_name(prayer);
    let title = non_empty(template.and_then(|t| t.custom_title.as_deref()))
        .map_or_else(|| name.clone(), str::to_owned);
    let body = non_empty(template.and_then(|t| t.custom_body.as_deref())).map_or_else(
        || ot_i18n::text_with("prayer-toast-body", &[("prayer", name.clone().into())]),
        str::to_owned,
    );
    let image = template
        .and_then(|t| t.image.clone())
        .or_else(|| default_prayer_image(prayer).map(str::to_owned));
    let content = ot_toast::Content {
        title,
        body,
        app_name: ot_i18n::text("toast-app-name"),
        image,
        accent: template
            .and_then(|t| t.accent_color.as_deref())
            .and_then(parse_accent),
        timestamp,
        note,
        action_labels: action_labels(),
        ..ot_toast::Content::default()
    };
    let mut spec = prefs.spec(content);
    // Dismiss only: an Azan is not put off.
    spec.behaviour.actions = 1;
    spec.behaviour.persistent = true;
    spec.behaviour.duration_secs = ot_toast::spec::PERSISTENT_CAP_SECS;
    spec
}

/// The reminder ahead of a prayer: the prayer's name and "in 10 minutes", with
/// the prayer's own picture and accent. A plain toast — it times out, plays no
/// sound and is held back by a pause, unlike the prayer itself.
pub fn prayer_before_spec(
    prefs: &ot_toast::Prefs,
    prayer: &str,
    minutes: u32,
    template: Option<&Template>,
    timestamp: Option<String>,
) -> ot_toast::ToastSpec {
    let name = crate::prayer_pane::prayer_name(prayer);
    let content = ot_toast::Content {
        title: name.clone(),
        body: ot_i18n::text_with_count("prayer-before-body", minutes, &[("prayer", name.into())]),
        app_name: ot_i18n::text("toast-app-name"),
        image: template
            .and_then(|t| t.image.clone())
            .or_else(|| default_prayer_image(prayer).map(str::to_owned)),
        accent: template
            .and_then(|t| t.accent_color.as_deref())
            .and_then(parse_accent),
        timestamp,
        action_labels: action_labels(),
        ..ot_toast::Content::default()
    };
    let mut spec = prefs.spec(content);
    spec.behaviour.actions = 1;
    spec
}

/// The built-in masjid picture a prayer shows until its template chooses one.
/// Syuruk has none of its own and borrows Dhuha's.
pub fn default_prayer_image(prayer: &str) -> Option<&'static str> {
    Some(match prayer {
        "fajr" => "@builtin/masjid-subuh",
        "syuruk" => "@builtin/masjid-dhuha",
        "dhuhr" | "jumuah" => "@builtin/masjid-zohor",
        "asr" => "@builtin/masjid-asar",
        "maghrib" => "@builtin/masjid-maghrib",
        "isha" => "@builtin/masjid-isyak",
        _ => return None,
    })
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
    fn at_the_limit_the_toast_offers_only_dismiss_and_says_so() {
        let _turn = LANGUAGE.lock().unwrap_or_else(|e| e.into_inner());
        let mut prefs = ot_toast::Prefs::default();
        prefs.behaviour.actions = 3;
        let typed = reminder("Ubat", None);
        let below = spec(&prefs, &typed, None, None, None, 9, 10);
        assert_eq!(below.actions().len(), 3);
        assert!(below.enabled(ot_toast::Action::Snooze));
        assert_eq!(below.content.note, None);
        let offset = spec(
            &prefs,
            &typed,
            None,
            None,
            Some("1 day before".to_owned()),
            0,
            10,
        );
        assert_eq!(
            offset.content.note.as_deref(),
            Some("1 day before"),
            "an offset fire says how far ahead it is"
        );

        for (language, line) in [
            (Language::English, "No more snoozes for this reminder."),
            (Language::Malay, "Tiada lagi tundaan untuk peringatan ini."),
        ] {
            in_language(language);
            let at = spec(
                &prefs,
                &typed,
                None,
                None,
                Some("1 day before".to_owned()),
                10,
                10,
            );
            assert_eq!(at.actions(), &[ot_toast::Action::Dismiss]);
            assert_eq!(at.content.note.as_deref(), Some(line));
        }
    }

    #[test]
    fn a_prayers_toast_is_persistent_dismiss_only_and_carries_both_names() {
        let _turn = LANGUAGE.lock().unwrap_or_else(|e| e.into_inner());
        for (language, names) in [
            (Language::English, ("Fajr", "Subuh")),
            (Language::Malay, ("Subuh", "Fajr")),
        ] {
            in_language(language);
            let spec = prayer_spec(
                &ot_toast::Prefs::default(),
                "fajr",
                None,
                Some("5:49 am".into()),
                None,
            );
            assert!(spec.behaviour.persistent, "{language:?}");
            assert_eq!(
                spec.behaviour.duration_secs,
                ot_toast::spec::PERSISTENT_CAP_SECS
            );
            assert_eq!(spec.behaviour.actions, 1, "Dismiss, and nothing to snooze");
            assert!(spec.content.title.contains(names.0) && spec.content.title.contains(names.1));
            assert!(
                spec.content.body.contains(&spec.content.title),
                "the body names the prayer: {}",
                spec.content.body
            );
            assert_eq!(spec.clone().clamped(), spec, "the cap survives clamping");
        }
        // A key nobody knows shows as itself rather than as nothing.
        assert!(!crate::prayer_pane::prayer_name("fajr").is_empty());
        assert_eq!(crate::prayer_pane::prayer_name("imsak"), "imsak");
    }

    fn prayer_template(title: Option<&str>, body: Option<&str>, accent: Option<&str>) -> Template {
        Template {
            id: 21,
            prayer_key: Some("fajr".to_owned()),
            custom_title: title.map(str::to_owned),
            custom_body: body.map(str::to_owned),
            accent_color: accent.map(str::to_owned),
            ..Template::default()
        }
    }

    #[test]
    fn a_prayer_templates_own_text_wins_and_empty_fields_fall_back_in_the_active_language() {
        let _turn = LANGUAGE.lock().unwrap_or_else(|e| e.into_inner());
        let prefs = ot_toast::Prefs::default();
        let written = prayer_template(
            Some("Bangun, Subuh dah masuk"),
            Some("Jangan lewat."),
            Some("#3b9eff"),
        );
        for language in [Language::English, Language::Malay] {
            in_language(language);
            let own = prayer_spec(&prefs, "fajr", Some(&written), None, None);
            assert_eq!(own.content.title, "Bangun, Subuh dah masuk", "{language:?}");
            assert_eq!(own.content.body, "Jangan lewat.");
            assert!(own.content.accent.is_some(), "the template's accent");

            // Nothing written: the built-in text, in the language in force.
            let blank = prayer_template(None, None, None);
            let built_in = prayer_spec(&prefs, "fajr", Some(&blank), None, None);
            let name = crate::prayer_pane::prayer_name("fajr");
            assert_eq!(built_in.content.title, name);
            assert!(built_in.content.body.contains(&name));
            assert!(
                built_in.content.accent.is_none(),
                "the global accent applies"
            );
            // No template at all is the same as an empty one.
            assert_eq!(prayer_spec(&prefs, "fajr", None, None, None), built_in);
        }
    }

    #[test]
    fn the_fire_and_the_preview_build_the_same_toast_and_it_carries_the_source_label() {
        let prefs = ot_toast::Prefs::default();
        let template = prayer_template(Some("Subuh"), None, None);
        let label = Some("Source: JAKIM e-Solat".to_owned());
        let fire = prayer_spec(
            &prefs,
            "fajr",
            Some(&template),
            Some("5:49 am".into()),
            label.clone(),
        );
        let preview = prayer_spec(
            &prefs,
            "fajr",
            Some(&template),
            Some("5:49 am".into()),
            label.clone(),
        );
        assert_eq!(fire, preview, "one function, so one toast");
        assert_eq!(fire.content.note, label, "the source label is on it");
    }

    #[test]
    fn every_prayer_has_a_built_in_picture_that_exists() {
        for prayer in ot_store::reminders::PRAYERS {
            let key = default_prayer_image(prayer).unwrap_or_else(|| panic!("{prayer}"));
            assert!(
                ot_store::builtin::find(key).is_some(),
                "{key} is not built in"
            );
        }
        assert_eq!(default_prayer_image("imsak"), None);
    }

    #[test]
    fn the_prayer_toast_strings_exist_in_both_catalogues() {
        for tag in ["en-US", "ms-MY"] {
            let keys = ot_i18n::message_keys(tag).expect("parses");
            for key in [
                "prayer-toast-body",
                "prayer-toast-unavailable-title",
                "prayer-toast-unavailable-body",
            ] {
                assert!(keys.contains(key), "{key} missing from {tag}");
            }
        }
    }

    #[test]
    fn an_alarm_waits_up_to_the_cap_and_a_task_keeps_the_display_duration() {
        let mut prefs = ot_toast::Prefs::default();
        prefs.behaviour.duration_secs = 12.0;
        let typed = reminder("x", None);

        let alarm = spec(
            &prefs,
            &typed,
            Some(&template("reminder-alarm")),
            None,
            None,
            0,
            10,
        );
        assert!(alarm.behaviour.persistent);
        assert_eq!(
            alarm.behaviour.duration_secs,
            ot_toast::spec::PERSISTENT_CAP_SECS
        );
        assert_eq!(alarm.clone().clamped(), alarm, "the cap survives clamping");

        for key in ["reminder-task", "reminder-timer", "reminder-appointment"] {
            let other = spec(&prefs, &typed, Some(&template(key)), None, None, 0, 10);
            assert!(!other.behaviour.persistent, "{key}");
            assert_eq!(other.behaviour.duration_secs, 12.0, "{key}");
        }
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
