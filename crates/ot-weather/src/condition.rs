//! From a WMO weather code to a condition class and its Fluent key
//! (FEAT-0066, `docs/specs/weather.md` §4 and §5).
//!
//! The service sends only numbers, so every word the user sees is ours and both
//! languages are covered by the `weather-` keys. The classes are the six the spec
//! names, plus [`Condition::Unknown`] for a code this app has not met: it gets a
//! generic key, and a log line so an added code is found rather than shown as an
//! empty toast.

/// The weather as the app tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Condition {
    Clear,
    Cloudy,
    Rain,
    Storm,
    Snow,
    Fog,
    /// A code the app does not know.
    Unknown,
}

impl Condition {
    /// The six real classes, in the order the settings list them.
    pub const ALL: [Condition; 6] = [
        Condition::Clear,
        Condition::Cloudy,
        Condition::Rain,
        Condition::Storm,
        Condition::Snow,
        Condition::Fog,
    ];

    /// The Fluent key of its name, which is also what a template for it is stored
    /// under (never renamed: `.claude/rules/i18n.md`).
    pub fn key(self) -> &'static str {
        match self {
            Condition::Clear => "weather-clear",
            Condition::Cloudy => "weather-cloudy",
            Condition::Rain => "weather-rain",
            Condition::Storm => "weather-storm",
            Condition::Snow => "weather-snow",
            Condition::Fog => "weather-fog",
            Condition::Unknown => "weather-unknown",
        }
    }
}

/// The class of a WMO weather code: the 29 Open-Meteo documents
/// (ADR-023), grouped by what a person would say. Mainly clear is clear, partly
/// cloudy and overcast are cloudy, drizzle and freezing rain are rain, snow grains
/// are snow, and every thunderstorm, with or without hail, is a storm.
pub fn classify(code: u8) -> Condition {
    match code {
        0 | 1 => Condition::Clear,
        2 | 3 => Condition::Cloudy,
        45 | 48 => Condition::Fog,
        51 | 53 | 55 | 56 | 57 | 61 | 63 | 65 | 66 | 67 | 80..=82 => Condition::Rain,
        71 | 73 | 75 | 77 | 85 | 86 => Condition::Snow,
        95 | 96 | 97 | 99 => Condition::Storm,
        other => {
            tracing::warn!(code = other, "an unknown weather code was met");
            Condition::Unknown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::{Arc, Mutex};

    /// Every code Open-Meteo documents (`https://open-meteo.com/en/docs`, read
    /// 2026-11-26), with the class each belongs to.
    const DOCUMENTED: [(u8, Condition); 29] = [
        (0, Condition::Clear),
        (1, Condition::Clear),
        (2, Condition::Cloudy),
        (3, Condition::Cloudy),
        (45, Condition::Fog),
        (48, Condition::Fog),
        (51, Condition::Rain),
        (53, Condition::Rain),
        (55, Condition::Rain),
        (56, Condition::Rain),
        (57, Condition::Rain),
        (61, Condition::Rain),
        (63, Condition::Rain),
        (65, Condition::Rain),
        (66, Condition::Rain),
        (67, Condition::Rain),
        (71, Condition::Snow),
        (73, Condition::Snow),
        (75, Condition::Snow),
        (77, Condition::Snow),
        (80, Condition::Rain),
        (81, Condition::Rain),
        (82, Condition::Rain),
        (85, Condition::Snow),
        (86, Condition::Snow),
        (95, Condition::Storm),
        (96, Condition::Storm),
        (97, Condition::Storm),
        (99, Condition::Storm),
    ];

    #[test]
    fn every_documented_code_maps_to_its_class_and_none_is_unknown() {
        for (code, expected) in DOCUMENTED {
            assert_eq!(classify(code), expected, "code {code}");
        }
    }

    #[test]
    fn every_class_has_a_distinct_key_and_the_unknown_one_its_own() {
        let mut keys: Vec<&str> = Condition::ALL.iter().map(|c| c.key()).collect();
        keys.push(Condition::Unknown.key());
        assert!(keys.iter().all(|k| k.starts_with("weather-")));
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len(), "{keys:?}");
    }

    #[test]
    fn the_keys_exist_in_both_locale_files_with_text_in_both() {
        let en = include_str!("../../../locales/en-US/omni_toaster.ftl");
        let ms = include_str!("../../../locales/ms-MY/omni_toaster.ftl");
        let mut all: Vec<Condition> = Condition::ALL.to_vec();
        all.push(Condition::Unknown);
        for condition in all {
            let key = condition.key();
            for (tag, text) in [("en-US", en), ("ms-MY", ms)] {
                let line = text
                    .lines()
                    .find(|line| line.starts_with(&format!("{key} =")))
                    .unwrap_or_else(|| panic!("{key} is missing from {tag}"));
                let words = line.split_once('=').map_or("", |(_, rest)| rest.trim());
                assert!(!words.is_empty(), "{key} is empty in {tag}");
            }
        }
    }

    #[derive(Clone, Default)]
    struct Captured(Arc<Mutex<Vec<u8>>>);

    impl io::Write for Captured {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().expect("lock").extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Captured {
        type Writer = Captured;
        fn make_writer(&'a self) -> Captured {
            self.clone()
        }
    }

    #[test]
    fn an_unknown_code_is_the_generic_class_and_is_logged() {
        let captured = Captured::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(captured.clone())
            .with_ansi(false)
            .finish();
        let condition = tracing::subscriber::with_default(subscriber, || classify(250));
        assert_eq!(condition, Condition::Unknown);
        let log = String::from_utf8(captured.0.lock().expect("lock").clone()).expect("utf8");
        assert!(
            log.contains("unknown weather code") && log.contains("250"),
            "{log}"
        );
    }

    #[test]
    fn a_known_code_logs_nothing() {
        let captured = Captured::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(captured.clone())
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, || classify(61));
        assert!(captured.0.lock().expect("lock").is_empty());
    }
}
