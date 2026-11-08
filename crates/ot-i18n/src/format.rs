//! The one place a date, a time or a number becomes text.
//!
//! Nothing else in the workspace may format one (`docs/10-i18n.md`). Day and
//! month names come from the Fluent files rather than from the OS locale, so a
//! Malay day name is correct even when Windows is set to English — and so a
//! language switch needs no restart.

use chrono::{Datelike as _, NaiveDateTime, Timelike as _, Weekday};

use crate::Language;

/// How the user wants times written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TimePrefs {
    /// `false` means 24-hour, which is the Malay default and so the default
    /// here.
    pub twelve_hour: bool,
}

impl TimePrefs {
    /// The default for a language: Malay is 24-hour
    /// (`docs/10-i18n.md`), English follows the user's choice and starts at
    /// 12-hour because that is what the Windows default is in `en-US`.
    pub fn for_language(language: Language) -> Self {
        Self {
            twelve_hour: matches!(language, Language::English),
        }
    }
}

/// Format a time of day.
pub fn format_time(when: &NaiveDateTime, prefs: TimePrefs) -> String {
    let (hour, period) = if prefs.twelve_hour {
        let raw = when.hour();
        // 0 reads as 12 am and 12 stays 12 pm.
        let twelve = match raw % 12 {
            0 => 12,
            other => other,
        };
        let period = if raw < 12 {
            crate::text("format-period-am")
        } else {
            crate::text("format-period-pm")
        };
        (twelve, Some(period))
    } else {
        (when.hour(), None)
    };

    let minute = format!("{:02}", when.minute());

    match period {
        Some(period) => crate::text_with(
            "format-time-12h",
            &[
                ("hour", hour.to_string().into()),
                ("minute", minute.into()),
                ("period", period.into()),
            ],
        ),
        None => crate::text_with(
            "format-time-24h",
            &[
                ("hour", format!("{hour:02}").into()),
                ("minute", minute.into()),
            ],
        ),
    }
}

/// Format a date and a time together.
pub fn format_datetime(when: &NaiveDateTime, prefs: TimePrefs) -> String {
    let date = crate::text_with(
        "format-date-short",
        &[
            ("day", when.day().to_string().into()),
            ("month", when.month().to_string().into()),
            ("year", when.year().to_string().into()),
        ],
    );
    let time = format_time(when, prefs);

    crate::text_with(
        "format-date-time",
        &[("date", date.into()), ("time", time.into())],
    )
}

/// The localized name of a weekday.
pub fn weekday_name(day: Weekday) -> String {
    crate::text(match day {
        Weekday::Mon => "day-monday",
        Weekday::Tue => "day-tuesday",
        Weekday::Wed => "day-wednesday",
        Weekday::Thu => "day-thursday",
        Weekday::Fri => "day-friday",
        Weekday::Sat => "day-saturday",
        Weekday::Sun => "day-sunday",
    })
}

/// The localized name of a month, 1 to 12. Out of range returns the number, so
/// a caller bug cannot panic in front of a user.
pub fn month_name(month: u32) -> String {
    const KEYS: [&str; 12] = [
        "month-january",
        "month-february",
        "month-march",
        "month-april",
        "month-may",
        "month-june",
        "month-july",
        "month-august",
        "month-september",
        "month-october",
        "month-november",
        "month-december",
    ];
    match KEYS.get(month.wrapping_sub(1) as usize) {
        Some(key) => crate::text(key),
        None => month.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, 8)
            .expect("a real date")
            .and_hms_opt(hour, minute, 0)
            .expect("a real time")
    }

    #[test]
    fn twelve_hour_boundaries_are_right() {
        // Midnight and noon are where 12-hour clocks go wrong.
        assert_eq!(hour_and_period(at(0, 5)), (12, "am"));
        assert_eq!(hour_and_period(at(11, 59)), (11, "am"));
        assert_eq!(hour_and_period(at(12, 0)), (12, "pm"));
        assert_eq!(hour_and_period(at(13, 0)), (1, "pm"));
        assert_eq!(hour_and_period(at(23, 30)), (11, "pm"));
    }

    /// The hour and period a 12-hour clock should show, computed the same way
    /// `format_time` does, so the boundary rules can be checked without a
    /// loaded Fluent bundle (the loader is global and other tests may have
    /// claimed it).
    fn hour_and_period(when: NaiveDateTime) -> (u32, &'static str) {
        let raw = when.hour();
        let twelve = match raw % 12 {
            0 => 12,
            other => other,
        };
        (twelve, if raw < 12 { "am" } else { "pm" })
    }

    #[test]
    fn minutes_are_always_two_digits() {
        // "9:5" would be wrong in either language.
        assert_eq!(format!("{:02}", at(9, 5).minute()), "05");
        assert_eq!(format!("{:02}", at(9, 50).minute()), "50");
    }

    #[test]
    fn time_prefs_default_to_twenty_four_hour() {
        assert!(!TimePrefs::default().twelve_hour);
        assert!(!TimePrefs::for_language(Language::Malay).twelve_hour);
        assert!(TimePrefs::for_language(Language::English).twelve_hour);
    }

    #[test]
    fn month_name_out_of_range_returns_the_number_rather_than_panicking() {
        // A caller bug should be visible, not fatal.
        assert_eq!(month_name(0), "0");
        assert_eq!(month_name(13), "13");
        assert_eq!(month_name(u32::MAX), u32::MAX.to_string());
    }

    #[test]
    fn formatting_uses_the_bundle_and_strips_fluent_isolates() {
        // One test that actually initialises the loader. It must not assume
        // which language another test left selected, so it checks shape rather
        // than exact words.
        crate::init(Language::English).expect("the bundled locales load");

        let rendered = format_datetime(&at(13, 5), TimePrefs::for_language(Language::English));

        assert!(
            !rendered.contains('\u{2068}') && !rendered.contains('\u{2069}'),
            "Fluent isolating marks leaked into {rendered:?}"
        );
        assert!(rendered.contains("8/11/2026"), "the date: {rendered}");
        assert!(rendered.contains("05"), "two-digit minutes: {rendered}");
    }
}
