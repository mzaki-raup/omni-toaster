//! A reminder's offsets and repeat, said in the user's language through the
//! one formatter in `ot-i18n` (`docs/10-i18n.md`, FEAT-0047). This only maps
//! the stored types onto the formatter's; every word comes from Fluent.

use ot_i18n::format::Unit;
use ot_store::reminders::{Offset, OffsetUnit, Recurrence, RecurrenceUnit, Weekday};

/// "1 day before".
pub fn offset(offset: Offset) -> String {
    let unit = match offset.unit {
        OffsetUnit::Minutes => Unit::Minutes,
        OffsetUnit::Hours => Unit::Hours,
        OffsetUnit::Days => Unit::Days,
        OffsetUnit::Weeks => Unit::Weeks,
    };
    ot_i18n::format::before(offset.amount, unit)
}

/// "every 2 weeks on Monday and Thursday".
pub fn repeat(recurrence: &Recurrence) -> String {
    let unit = match recurrence.unit {
        RecurrenceUnit::Days => Unit::Days,
        RecurrenceUnit::Weeks => Unit::Weeks,
        RecurrenceUnit::Months => Unit::Months,
        RecurrenceUnit::Years => Unit::Years,
    };
    let days: Vec<jiff::civil::Weekday> = recurrence.weekdays.iter().map(|d| weekday(*d)).collect();
    ot_i18n::format::every(recurrence.every.max(1), unit, &days)
}

/// The stored weekday as jiff's.
pub fn weekday(day: Weekday) -> jiff::civil::Weekday {
    match day {
        Weekday::Monday => jiff::civil::Weekday::Monday,
        Weekday::Tuesday => jiff::civil::Weekday::Tuesday,
        Weekday::Wednesday => jiff::civil::Weekday::Wednesday,
        Weekday::Thursday => jiff::civil::Weekday::Thursday,
        Weekday::Friday => jiff::civil::Weekday::Friday,
        Weekday::Saturday => jiff::civil::Weekday::Saturday,
        Weekday::Sunday => jiff::civil::Weekday::Sunday,
    }
}
