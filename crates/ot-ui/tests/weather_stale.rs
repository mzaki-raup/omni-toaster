//! The offline stale marker's words, in both languages (FEAT-0071,
//! `docs/specs/weather.md` §7).
//!
//! One test in its own process, because it switches the process-wide language.

use jiff::SignedDuration;
use ot_i18n::Language;
use ot_ui::weather_pane::age_text;

const KEYS: [&str; 10] = [
    "weather-now-heading",
    "weather-now-label",
    "weather-now",
    "weather-updated",
    "weather-stale",
    "weather-now-none",
    "weather-now-off",
    "weather-age-minutes",
    "weather-age-hours",
    "weather-age-days",
];

fn ages() -> Vec<String> {
    [0, 1, 2, 59, 60, 119, 180, 1439, 1440, 3 * 1440]
        .into_iter()
        .map(|minutes| age_text(SignedDuration::from_mins(minutes)))
        .collect()
}

fn stale(age: &str) -> String {
    ot_i18n::text_with("weather-stale", &[("age", age.to_owned().into())])
}

#[test]
fn how_old_the_weather_is_and_the_stale_marker_read_right_in_both_languages() {
    for tag in ["en-US", "ms-MY"] {
        let keys = ot_i18n::message_keys(tag).expect("parses");
        for key in KEYS.into_iter().chain(["weather-age-now"]) {
            assert!(keys.contains(key), "{key} missing from {tag}");
        }
    }

    ot_i18n::init(Language::English).expect("installs");
    assert_eq!(
        ages(),
        [
            "just now",
            "1 minute ago",
            "2 minutes ago",
            "59 minutes ago",
            "1 hour ago",
            "1 hour ago",
            "3 hours ago",
            "23 hours ago",
            "1 day ago",
            "3 days ago"
        ]
    );
    assert_eq!(
        stale("3 hours ago"),
        "Not up to date: showing the weather from 3 hours ago."
    );
    assert_eq!(
        ot_i18n::text_with(
            "weather-now",
            &[("condition", "Rain".into()), ("temperature", "25".into())]
        ),
        "Rain, 25 °C"
    );
    assert_eq!(
        ot_i18n::text_with("weather-updated", &[("age", "just now".into())]),
        "Updated just now."
    );

    ot_i18n::set_language(Language::Malay).expect("switches");
    assert_eq!(
        ages(),
        [
            "sebentar tadi",
            "1 minit lalu",
            "2 minit lalu",
            "59 minit lalu",
            "1 jam lalu",
            "1 jam lalu",
            "3 jam lalu",
            "23 jam lalu",
            "1 hari lalu",
            "3 hari lalu"
        ]
    );
    assert_eq!(
        stale("3 jam lalu"),
        "Tidak terkini: memaparkan cuaca dari 3 jam lalu."
    );
    assert_eq!(
        ot_i18n::text_with(
            "weather-now",
            &[("condition", "Hujan".into()), ("temperature", "25".into())]
        ),
        "Hujan, 25 °C"
    );
}
