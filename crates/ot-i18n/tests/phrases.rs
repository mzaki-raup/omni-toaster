//! Every phrase the formatter writes for a reminder, in both languages,
//! against a written table (FEAT-0047): offsets, repeats, lists, and a date
//! and time in each language, 12- and 24-hour, day first and month first.
//!
//! One test in its own process, because it switches the process-wide
//! language (`tests/switch.rs` says why).

use jiff::civil::{Weekday, date};
use ot_i18n::Language;
use ot_i18n::format::{
    Clock, Regional, TimePrefs, Unit, before, every, format_datetime, format_time, list,
};

#[test]
fn every_phrase_reads_as_the_table_says_in_both_languages() {
    ot_i18n::init(Language::English).expect("the first install succeeds");
    let names = |days: &[&str]| days.iter().map(|d| (*d).to_owned()).collect::<Vec<_>>();
    let at = date(2026, 3, 7).at(21, 5, 0, 0);
    let us = Regional {
        month_first: Some(true),
        twenty_four_hour: Some(false),
    };
    let uk = Regional {
        month_first: Some(false),
        twenty_four_hour: Some(true),
    };

    // English.
    assert_eq!(before(1, Unit::Days), "1 day before");
    assert_eq!(before(2, Unit::Days), "2 days before");
    assert_eq!(before(1, Unit::Minutes), "1 minute before");
    assert_eq!(before(15, Unit::Minutes), "15 minutes before");
    assert_eq!(before(1, Unit::Hours), "1 hour before");
    assert_eq!(before(3, Unit::Weeks), "3 weeks before");
    assert_eq!(every(1, Unit::Days, &[]), "every day");
    assert_eq!(every(3, Unit::Days, &[]), "every 3 days");
    assert_eq!(every(1, Unit::Weeks, &[]), "every week");
    assert_eq!(
        every(2, Unit::Weeks, &[Weekday::Monday, Weekday::Thursday]),
        "every 2 weeks on Monday and Thursday"
    );
    assert_eq!(
        every(
            1,
            Unit::Weeks,
            &[Weekday::Monday, Weekday::Tuesday, Weekday::Thursday]
        ),
        "every week on Monday, Tuesday and Thursday"
    );
    assert_eq!(every(1, Unit::Months, &[]), "every month");
    assert_eq!(every(6, Unit::Months, &[]), "every 6 months");
    assert_eq!(every(1, Unit::Years, &[]), "every year");
    assert_eq!(list(&names(&["A"])), "A");
    assert_eq!(list(&[]), "");
    assert_eq!(
        format_datetime(
            &at,
            TimePrefs::resolve(Language::English, Clock::Automatic, us)
        ),
        "3/7/2026 at 9:05 pm",
        "month first and 12-hour, as Windows says in the US"
    );
    assert_eq!(
        format_datetime(
            &at,
            TimePrefs::resolve(Language::English, Clock::Automatic, uk)
        ),
        "7/3/2026 at 21:05",
        "day first and 24-hour, as Windows says in the UK"
    );
    assert_eq!(
        format_time(
            &at,
            TimePrefs::resolve(Language::English, Clock::TwentyFourHour, us)
        ),
        "21:05",
        "the user's 24-hour wins over Windows"
    );

    // Malay.
    ot_i18n::set_language(Language::Malay).expect("switches");
    assert_eq!(ot_i18n::current_language(), Some(Language::Malay));
    assert_eq!(before(1, Unit::Days), "1 hari sebelum");
    assert_eq!(before(15, Unit::Minutes), "15 minit sebelum");
    assert_eq!(before(2, Unit::Weeks), "2 minggu sebelum");
    assert_eq!(every(1, Unit::Days, &[]), "setiap hari");
    assert_eq!(every(3, Unit::Days, &[]), "setiap 3 hari");
    assert_eq!(
        every(2, Unit::Weeks, &[Weekday::Monday, Weekday::Thursday]),
        "setiap 2 minggu pada Isnin dan Khamis"
    );
    assert_eq!(
        every(
            1,
            Unit::Weeks,
            &[Weekday::Monday, Weekday::Tuesday, Weekday::Thursday]
        ),
        "setiap minggu pada Isnin, Selasa dan Khamis"
    );
    assert_eq!(every(1, Unit::Months, &[]), "setiap bulan");
    assert_eq!(every(1, Unit::Years, &[]), "setiap tahun");
    for regional in [us, uk] {
        assert_eq!(
            format_datetime(
                &at,
                TimePrefs::resolve(Language::Malay, Clock::Automatic, regional)
            ),
            "7/3/2026 pada 21:05",
            "Malay is d/m/yyyy and 24-hour, whatever Windows says"
        );
    }
    assert_eq!(
        format_time(
            &at,
            TimePrefs::resolve(Language::Malay, Clock::TwelveHour, us)
        ),
        "9:05 petang",
        "unless the user chose 12-hour"
    );
}
