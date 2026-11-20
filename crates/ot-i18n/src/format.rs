//! The one place a date, a time or a number becomes text.
//!
//! Nothing else in the workspace may format one (`docs/10-i18n.md`). Day and
//! month names come from the Fluent files rather than from the OS locale, so a
//! Malay day name is correct even when Windows is set to English — and so a
//! language switch needs no restart.

use std::sync::{PoisonError, RwLock};

use jiff::civil::{DateTime, Weekday};

use crate::Language;

/// How the user wants times written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TimePrefs {
    /// `false` means 24-hour, which is the Malay default and so the default
    /// here.
    pub twelve_hour: bool,
    /// The date is month first, `m/d/yyyy`. Only ever true in English, and
    /// only when Windows' regional setting is month first.
    pub month_first: bool,
}

/// The clock the user chose in Settings ▸ General.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Clock {
    /// 24-hour in Malay; in English, whatever Windows' regional setting is.
    #[default]
    Automatic,
    TwelveHour,
    TwentyFourHour,
}

impl Clock {
    /// From the settings file's value; anything unknown is Automatic.
    pub fn parse(text: &str) -> Self {
        match text {
            "12" => Self::TwelveHour,
            "24" => Self::TwentyFourHour,
            _ => Self::Automatic,
        }
    }

    /// The settings file's value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::TwelveHour => "12",
            Self::TwentyFourHour => "24",
        }
    }

    /// The Fluent key naming it in Settings ▸ General.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::Automatic => "settings-clock-automatic",
            Self::TwelveHour => "settings-clock-12",
            Self::TwentyFourHour => "settings-clock-24",
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::Automatic, Self::TwelveHour, Self::TwentyFourHour]
    }
}

/// What Windows' regional settings say, as `ot-platform` read them. `None`
/// where Windows could not say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Regional {
    pub month_first: Option<bool>,
    pub twenty_four_hour: Option<bool>,
}

/// The user's clock and Windows' regional answer, held here like the active
/// language, so every caller formats the same way without passing them
/// around.
static PREFS: RwLock<(Clock, Regional)> = RwLock::new((
    Clock::Automatic,
    Regional {
        month_first: None,
        twenty_four_hour: None,
    },
));

/// Set the clock the user chose.
pub fn set_clock(clock: Clock) {
    PREFS.write().unwrap_or_else(PoisonError::into_inner).0 = clock;
}

/// Record what Windows' regional settings say.
pub fn set_regional(regional: Regional) {
    PREFS.write().unwrap_or_else(PoisonError::into_inner).1 = regional;
}

impl TimePrefs {
    /// The defaults for a language, with Automatic and no regional answer:
    /// Malay 24-hour and day first; English 12-hour, as the Windows default
    /// in `en-US` is, and day first.
    pub fn for_language(language: Language) -> Self {
        Self::resolve(language, Clock::Automatic, Regional::default())
    }

    /// The rule (`docs/10-i18n.md`): Malay is `d/m/yyyy` and 24-hour unless
    /// the user chose 12-hour; English follows Windows' date order, and its
    /// clock unless the user chose one.
    pub fn resolve(language: Language, clock: Clock, regional: Regional) -> Self {
        let english = matches!(language, Language::English);
        let twelve_hour = match clock {
            Clock::TwelveHour => true,
            Clock::TwentyFourHour => false,
            Clock::Automatic if english => !regional.twenty_four_hour.unwrap_or(false),
            Clock::Automatic => false,
        };
        Self {
            twelve_hour,
            month_first: english && regional.month_first.unwrap_or(false),
        }
    }

    /// For the active language, with the user's clock and Windows' answer.
    pub fn current() -> Self {
        let (clock, regional) = *PREFS.read().unwrap_or_else(PoisonError::into_inner);
        Self::resolve(
            crate::current_language().unwrap_or(Language::English),
            clock,
            regional,
        )
    }
}

/// A unit of an offset or a repeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Minutes,
    Hours,
    Days,
    Weeks,
    Months,
    Years,
}

impl Unit {
    fn suffix(self) -> &'static str {
        match self {
            Self::Minutes => "minutes",
            Self::Hours => "hours",
            Self::Days => "days",
            Self::Weeks => "weeks",
            Self::Months => "months",
            Self::Years => "years",
        }
    }
}

/// "1 day before", "15 minutes before" — an offset, with its plural.
pub fn before(amount: u32, unit: Unit) -> String {
    crate::text_with_count(&format!("format-before-{}", unit.suffix()), amount, &[])
}

/// "every day", "every 2 weeks on Monday and Thursday" — a repeat, with its
/// plural and, for weeks, its weekdays.
pub fn every(count: u32, unit: Unit, weekdays: &[Weekday]) -> String {
    if unit == Unit::Weeks && !weekdays.is_empty() {
        let names: Vec<String> = weekdays.iter().map(|day| weekday_name(*day)).collect();
        return crate::text_with_count(
            "format-every-weeks-on",
            count,
            &[("days", list(&names).into())],
        );
    }
    crate::text_with_count(&format!("format-every-{}", unit.suffix()), count, &[])
}

/// "Monday", "Monday and Thursday", "Monday, Tuesday and Thursday" — each
/// language's own way of listing, never commas glued in code.
pub fn list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => crate::text_with(
            "format-list-pair",
            &[("first", first.into()), ("second", second.into())],
        ),
        [first, rest @ ..] => crate::text_with(
            "format-list-more",
            &[("first", first.into()), ("rest", list(rest).into())],
        ),
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
    let key = if prefs.month_first {
        "format-date-short-month-first"
    } else {
        "format-date-short"
    };
    let date = crate::text_with(
        key,
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
    fn the_clock_and_the_date_order_resolve_by_the_rule() {
        let us = Regional {
            month_first: Some(true),
            twenty_four_hour: Some(false),
        };
        let uk = Regional {
            month_first: Some(false),
            twenty_four_hour: Some(true),
        };
        let resolve = TimePrefs::resolve;
        // Malay: day first always; 24-hour unless the user chose 12.
        for regional in [us, uk, Regional::default()] {
            assert_eq!(
                resolve(Language::Malay, Clock::Automatic, regional),
                TimePrefs {
                    twelve_hour: false,
                    month_first: false
                }
            );
            assert!(resolve(Language::Malay, Clock::TwelveHour, regional).twelve_hour);
        }
        // English: Windows' order, and Windows' clock unless chosen.
        assert_eq!(
            resolve(Language::English, Clock::Automatic, us),
            TimePrefs {
                twelve_hour: true,
                month_first: true
            }
        );
        assert_eq!(
            resolve(Language::English, Clock::Automatic, uk),
            TimePrefs {
                twelve_hour: false,
                month_first: false
            }
        );
        assert!(!resolve(Language::English, Clock::TwentyFourHour, us).twelve_hour);
        assert!(resolve(Language::English, Clock::TwelveHour, uk).twelve_hour);
        // Windows silent: 12-hour, day first.
        assert_eq!(
            resolve(Language::English, Clock::Automatic, Regional::default()),
            TimePrefs {
                twelve_hour: true,
                month_first: false
            }
        );
    }

    #[test]
    fn the_clock_round_trips_through_its_settings_value() {
        for clock in Clock::all() {
            assert_eq!(Clock::parse(clock.as_str()), clock);
        }
        assert_eq!(Clock::parse("nonsense"), Clock::Automatic);
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
