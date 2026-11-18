//! The one place a date, a time or a number becomes text.
//!
//! Nothing else in the workspace may format one (`docs/10-i18n.md`). Day and
//! month names come from the Fluent files rather than from the OS locale, so a
//! Malay day name is correct even when Windows is set to English — and so a
//! language switch needs no restart.

use jiff::civil::{DateTime, Weekday};

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
pub fn format_time(when: &DateTime, prefs: TimePrefs) -> String {
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
pub fn format_datetime(when: &DateTime, prefs: TimePrefs) -> String {
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
        Weekday::Monday => "day-monday",
        Weekday::Tuesday => "day-tuesday",
        Weekday::Wednesday => "day-wednesday",
        Weekday::Thursday => "day-thursday",
        Weekday::Friday => "day-friday",
        Weekday::Saturday => "day-saturday",
        Weekday::Sunday => "day-sunday",
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

    fn at(hour: i8, minute: i8) -> DateTime {
        jiff::civil::date(2026, 11, 8).at(hour, minute, 0, 0)
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
    fn hour_and_period(when: DateTime) -> (i8, &'static str) {
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
    fn malaysian_time_resolves_from_the_bundled_zone_database() {
        // Windows has no IANA database; jiff bundles one (ADR-017). JAKIM's
        // times are Asia/Kuala_Lumpur whatever the PC's zone is, so this must
        // resolve on every machine, and be UTC+8.
        let midnight_utc: jiff::Timestamp = "2026-11-19T00:00:00Z".parse().expect("an instant");
        let kl = midnight_utc
            .in_tz("Asia/Kuala_Lumpur")
            .expect("Asia/Kuala_Lumpur is in the bundled database");
        assert_eq!(
            kl.datetime(),
            jiff::civil::date(2026, 11, 19).at(8, 0, 0, 0)
        );
        assert_eq!(kl.offset(), jiff::tz::offset(8));
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
        // Not `init(...).expect(...)`: the loader is a process-wide OnceLock
        // and tests run in parallel, so whichever test loses the race gets
        // AlreadyInitialised. Unwrapping made this test pass or fail on thread
        // scheduling — green here, red on CI.
        crate::ensure_initialised(Language::English);

        let rendered = format_datetime(&at(13, 5), TimePrefs::for_language(Language::English));

        assert!(
            !rendered.contains('\u{2068}') && !rendered.contains('\u{2069}'),
            "Fluent isolating marks leaked into {rendered:?}"
        );
        assert!(rendered.contains("8/11/2026"), "the date: {rendered}");
        assert!(rendered.contains("05"), "two-digit minutes: {rendered}");
    }
}
