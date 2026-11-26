//! When a reminder fires next: its recurrence and its offsets, computed in the
//! local wall clock (`docs/specs/reminders.md` §4, §7).
//!
//! Pure functions — no clock, no thread, no database. The scheduler hands in
//! the instant it is asking about and the zone to read the wall clock in, so
//! every rule here is tested with a fixed zone and a fixed time.
//!
//! - Occurrences are civil dates and times, stepped by calendar days, weeks,
//!   months or years from the reminder's own date — never by adding 24 hours —
//!   so "every day at 07:00" is 07:00 on both sides of a DST change.
//! - A monthly or yearly date the month does not have (the 31st, 29 February)
//!   falls on the month's last day, and the next month goes back to the
//!   original day: 31 January, 28 February, 31 March.
//! - A wall-clock time a DST gap skips fires when the gap ends; one a DST fold
//!   repeats fires once, on its first pass.
//! - Offsets in minutes and hours are elapsed time; offsets in days and weeks
//!   are calendar days, so "1 day before 07:00" is 07:00 the day before even
//!   when that day was 23 hours long.

use std::iter;

use jiff::civil::{Date, DateTime};
use jiff::tz::{AmbiguousOffset, TimeZone};
use jiff::{SignedDuration, Span, Timestamp};
use ot_store::reminders::{Offset, OffsetUnit, Recurrence, RecurrenceUnit, Trigger, Weekday};

/// One fire: when it happens, the occurrence it belongs to, and the offset
/// that put it ahead of that occurrence (`None` for the occurrence itself).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fire {
    pub at: Timestamp,
    pub occurrence: Timestamp,
    pub offset: Option<Offset>,
}

/// The first fire strictly after `after`, or `None` when the reminder has no
/// more.
///
/// An occurrence always fires at its own time; each offset before it is a fire
/// of its own as well. Two fires at the same instant — two equal offsets, or a
/// daily reminder whose 1-day offset lands on the next day's own fire — are
/// one fire, the first in occurrence order: one toast, not two at once.
pub fn next_fire(
    trigger: &Trigger,
    recurrence: Option<&Recurrence>,
    after: Timestamp,
    tz: &TimeZone,
) -> Option<Fire> {
    let (start, offsets) = match trigger {
        Trigger::At { at } => (*at, &[][..]),
        Trigger::OffsetBefore { at, offsets } => (*at, offsets.as_slice()),
        // Counted from the working day, not from a date: `crate::fatigue`.
        // The fatigue schedules count from the working day and a prayer's
        // times are given to the scheduler: neither is a date and a repeat.
        Trigger::Every { .. } | Trigger::WorkHours { .. } | Trigger::Prayer { .. } => {
            return None;
        }
    };
    let lead = offsets
        .iter()
        .map(|offset| longest_lead(*offset))
        .max()
        .unwrap_or(SignedDuration::ZERO);

    // An offset fire is never later than its occurrence, so only occurrences
    // after `after` can still have fires ahead.
    let from = after.to_zoned_datetime(tz.clone()).date();
    let occurrences = civil_occurrences(start, recurrence, from)
        .filter_map(|civil| resolve(civil, tz))
        .filter(|occurrence| *occurrence > after);

    let mut best: Option<Fire> = None;
    for occurrence in occurrences {
        if let Some(best) = best {
            // No fire of this occurrence or any later one can come first.
            let earliest_possible = occurrence.checked_sub(lead).unwrap_or(Timestamp::MIN);
            if earliest_possible > best.at {
                break;
            }
        }
        for fire in fires_of(occurrence, offsets, tz) {
            if fire.at > after && best.is_none_or(|best| fire.at < best.at) {
                best = Some(fire);
            }
        }
    }
    best
}

/// The occurrence itself and one fire per offset before it.
fn fires_of<'a>(
    occurrence: Timestamp,
    offsets: &'a [Offset],
    tz: &'a TimeZone,
) -> impl Iterator<Item = Fire> + 'a {
    offsets
        .iter()
        .filter_map(move |offset| {
            let at = before(occurrence, *offset, tz)?;
            Some(Fire {
                at,
                occurrence,
                offset: Some(*offset),
            })
        })
        .chain(iter::once(Fire {
            at: occurrence,
            occurrence,
            offset: None,
        }))
}

/// `occurrence` less `offset`: elapsed time for minutes and hours, calendar
/// days in `tz` for days and weeks. `None` past the end of time.
fn before(occurrence: Timestamp, offset: Offset, tz: &TimeZone) -> Option<Timestamp> {
    let amount = i64::from(offset.amount);
    match offset.unit {
        OffsetUnit::Minutes => occurrence
            .checked_sub(SignedDuration::from_mins(amount))
            .ok(),
        OffsetUnit::Hours => occurrence
            .checked_sub(SignedDuration::from_hours(amount))
            .ok(),
        OffsetUnit::Days | OffsetUnit::Weeks => {
            let span = match offset.unit {
                OffsetUnit::Weeks => Span::new().try_weeks(amount).ok()?,
                _ => Span::new().try_days(amount).ok()?,
            };
            let zoned = occurrence.to_zoned_datetime(tz.clone());
            Some(zoned.checked_sub(span).ok()?.timestamp())
        }
    }
}

/// An upper bound on how far ahead of its occurrence an offset can fire. A
/// calendar day is at most 25 hours, a DST change being at most an hour.
fn longest_lead(offset: Offset) -> SignedDuration {
    let amount = i64::from(offset.amount);
    match offset.unit {
        OffsetUnit::Minutes => SignedDuration::from_mins(amount),
        OffsetUnit::Hours => SignedDuration::from_hours(amount),
        OffsetUnit::Days => SignedDuration::from_hours(amount * 25),
        OffsetUnit::Weeks => SignedDuration::from_hours(amount * 7 * 25),
    }
}

/// The instant a wall-clock time names in `tz`. In a gap, the instant the gap
/// ends; in a fold, the first of the two.
fn resolve(civil: DateTime, tz: &TimeZone) -> Option<Timestamp> {
    let ambiguous = tz.to_ambiguous_timestamp(civil);
    match ambiguous.offset() {
        AmbiguousOffset::Gap { after, .. } => {
            // Read with the offset after the gap, the wall-clock time lands
            // just before the transition; the transition is the gap's end.
            let inside = after.to_timestamp(civil).ok()?;
            tz.following(inside)
                .next()
                .map(|transition| transition.timestamp())
        }
        AmbiguousOffset::Unambiguous { .. } | AmbiguousOffset::Fold { .. } => {
            ambiguous.earlier().ok()
        }
    }
}

/// Every occurrence of a reminder from `start`, as wall-clock times in order,
/// beginning a little before `from` so the caller skips only a few.
fn civil_occurrences(
    start: DateTime,
    recurrence: Option<&Recurrence>,
    from: Date,
) -> Box<dyn Iterator<Item = DateTime>> {
    let Some(recurrence) = recurrence else {
        return Box::new(iter::once(start));
    };
    let every = i64::from(recurrence.every);
    if every == 0 {
        return Box::new(iter::empty());
    }
    let time = start.time();
    let first = start.date();

    match recurrence.unit {
        RecurrenceUnit::Days => {
            let skip = (days_between(first, from) / every - 1).max(0);
            Box::new(
                (skip..)
                    .map_while(move |k| add_days(first, k.checked_mul(every)?))
                    .map(move |date| date.to_datetime(time)),
            )
        }
        RecurrenceUnit::Weeks => {
            let mut weekdays: Vec<i64> = if recurrence.weekdays.is_empty() {
                vec![i64::from(first.weekday().to_monday_zero_offset())]
            } else {
                recurrence
                    .weekdays
                    .iter()
                    .map(|day| monday_zero(*day))
                    .collect()
            };
            weekdays.sort_unstable();
            weekdays.dedup();
            let monday =
                first.checked_sub(Span::new().days(first.weekday().to_monday_zero_offset()));
            let Ok(monday) = monday else {
                return Box::new(iter::empty());
            };
            let skip = (days_between(monday, from) / (7 * every) - 1).max(0);
            Box::new(
                (skip..)
                    .map_while(move |k| {
                        let week = add_days(monday, k.checked_mul(7 * every)?)?;
                        Some(
                            weekdays
                                .clone()
                                .into_iter()
                                .filter_map(move |day| add_days(week, day))
                                .collect::<Vec<_>>(),
                        )
                    })
                    .flatten()
                    .filter(move |date| *date >= first)
                    .map(move |date| date.to_datetime(time)),
            )
        }
        RecurrenceUnit::Months | RecurrenceUnit::Years => {
            let step = match recurrence.unit {
                RecurrenceUnit::Years => every * 12,
                _ => every,
            };
            let months_apart = (i64::from(from.year()) - i64::from(first.year())) * 12
                + i64::from(from.month())
                - i64::from(first.month());
            let skip = (months_apart / step - 1).max(0);
            Box::new(
                (skip..)
                    .map_while(move |k| add_months_clamped(first, k.checked_mul(step)?))
                    .map(move |date| date.to_datetime(time)),
            )
        }
    }
}

/// `first` moved on by whole months, keeping its day of the month, or the
/// month's last day when the month is shorter.
fn add_months_clamped(first: Date, months: i64) -> Option<Date> {
    let index = i64::from(first.month()) - 1 + months;
    let year = i16::try_from(i64::from(first.year()) + index.div_euclid(12)).ok()?;
    let month = i8::try_from(index.rem_euclid(12) + 1).ok()?;
    let month_start = Date::new(year, month, 1).ok()?;
    let day = first.day().min(month_start.days_in_month());
    Date::new(year, month, day).ok()
}

fn add_days(date: Date, days: i64) -> Option<Date> {
    date.checked_add(Span::new().try_days(days).ok()?).ok()
}

/// Whole days from `a` to `b`; 0 if they cannot be compared.
fn days_between(a: Date, b: Date) -> i64 {
    a.until(b).map_or(0, |span| i64::from(span.get_days()))
}

fn monday_zero(day: Weekday) -> i64 {
    match day {
        Weekday::Monday => 0,
        Weekday::Tuesday => 1,
        Weekday::Wednesday => 2,
        Weekday::Thursday => 3,
        Weekday::Friday => 4,
        Weekday::Saturday => 5,
        Weekday::Sunday => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn new_york() -> TimeZone {
        TimeZone::get("America/New_York").expect("in the bundled database")
    }

    fn kuala_lumpur() -> TimeZone {
        TimeZone::get("Asia/Kuala_Lumpur").expect("in the bundled database")
    }

    fn at(civil: DateTime, tz: &TimeZone) -> Timestamp {
        tz.to_ambiguous_timestamp(civil)
            .unambiguous()
            .expect("an unambiguous time")
    }

    fn recurring(every: u32, unit: RecurrenceUnit) -> Recurrence {
        Recurrence {
            every,
            unit,
            weekdays: Vec::new(),
        }
    }

    /// The next `n` fires, each asked for strictly after the one before, as
    /// wall-clock times in `tz`.
    fn fires(
        trigger: &Trigger,
        recurrence: Option<&Recurrence>,
        after: Timestamp,
        tz: &TimeZone,
        n: usize,
    ) -> Vec<DateTime> {
        let mut out = Vec::new();
        let mut after = after;
        while out.len() < n {
            let Some(fire) = next_fire(trigger, recurrence, after, tz) else {
                break;
            };
            out.push(fire.at.to_zoned_datetime(tz.clone()).datetime());
            after = fire.at;
        }
        out
    }

    #[test]
    fn once_fires_at_its_time_and_never_again() {
        let tz = kuala_lumpur();
        let trigger = Trigger::At {
            at: date(2026, 12, 25).at(9, 0, 0, 0),
        };
        let before = at(date(2026, 12, 1).at(0, 0, 0, 0), &tz);
        let fire = next_fire(&trigger, None, before, &tz).expect("fires");
        assert_eq!(fire.at, at(date(2026, 12, 25).at(9, 0, 0, 0), &tz));
        assert_eq!(fire.offset, None);
        assert_eq!(
            next_fire(&trigger, None, fire.at, &tz),
            None,
            "strictly after its own time, a one-off has nothing left"
        );
    }

    #[test]
    fn every_day_at_seven_stays_seven_across_both_dst_changes() {
        let tz = new_york();
        let trigger = Trigger::At {
            at: date(2026, 3, 1).at(7, 0, 0, 0),
        };
        let daily = recurring(1, RecurrenceUnit::Days);
        for (from, expect) in [
            // Spring forward on Sunday 8 March 2026: that day is 23 hours long.
            (
                date(2026, 3, 6),
                [date(2026, 3, 7), date(2026, 3, 8), date(2026, 3, 9)],
            ),
            // Fall back on Sunday 1 November 2026: that day is 25 hours long.
            (
                date(2026, 10, 30),
                [date(2026, 10, 31), date(2026, 11, 1), date(2026, 11, 2)],
            ),
        ] {
            let after = at(from.at(12, 0, 0, 0), &tz);
            let got = fires(&trigger, Some(&daily), after, &tz, 3);
            let want: Vec<DateTime> = expect.iter().map(|d| d.at(7, 0, 0, 0)).collect();
            assert_eq!(got, want);
        }
    }

    #[test]
    fn a_time_a_gap_skips_fires_when_the_gap_ends() {
        // 02:30 does not exist in New York on 8 March 2026: the clock goes
        // from 01:59:59 EST to 03:00:00 EDT.
        let tz = new_york();
        let trigger = Trigger::At {
            at: date(2026, 3, 1).at(2, 30, 0, 0),
        };
        let daily = recurring(1, RecurrenceUnit::Days);
        let after = at(date(2026, 3, 7).at(12, 0, 0, 0), &tz);
        let fire = next_fire(&trigger, Some(&daily), after, &tz).expect("fires");
        assert_eq!(
            fire.at,
            "2026-03-08T07:00:00Z"
                .parse::<Timestamp>()
                .expect("instant")
        );
        assert_eq!(
            fire.at.to_zoned_datetime(tz.clone()).datetime(),
            date(2026, 3, 8).at(3, 0, 0, 0)
        );
        // And the day after is back to 02:30.
        let next = next_fire(&trigger, Some(&daily), fire.at, &tz).expect("fires");
        assert_eq!(
            next.at.to_zoned_datetime(tz.clone()).datetime(),
            date(2026, 3, 9).at(2, 30, 0, 0)
        );
    }

    #[test]
    fn a_time_a_fold_repeats_fires_once() {
        // 01:30 happens twice in New York on 1 November 2026, first in EDT and
        // then in EST. It fires on the first and not on the second.
        let tz = new_york();
        let trigger = Trigger::At {
            at: date(2026, 10, 1).at(1, 30, 0, 0),
        };
        let daily = recurring(1, RecurrenceUnit::Days);
        let after = at(date(2026, 10, 31).at(12, 0, 0, 0), &tz);
        let first = next_fire(&trigger, Some(&daily), after, &tz).expect("fires");
        assert_eq!(
            first.at,
            "2026-11-01T05:30:00Z"
                .parse::<Timestamp>()
                .expect("instant")
        );
        let second = next_fire(&trigger, Some(&daily), first.at, &tz).expect("fires");
        assert_eq!(
            second.at,
            "2026-11-02T06:30:00Z"
                .parse::<Timestamp>()
                .expect("instant"),
            "the next fire is the next day, not the repeated hour"
        );
    }

    #[test]
    fn the_31st_falls_on_each_months_last_day_and_comes_back() {
        let tz = kuala_lumpur();
        let trigger = Trigger::At {
            at: date(2027, 1, 31).at(9, 0, 0, 0),
        };
        let monthly = recurring(1, RecurrenceUnit::Months);
        let after = at(date(2027, 1, 1).at(0, 0, 0, 0), &tz);
        let got = fires(&trigger, Some(&monthly), after, &tz, 5);
        let want: Vec<DateTime> = [
            date(2027, 1, 31),
            date(2027, 2, 28),
            date(2027, 3, 31),
            date(2027, 4, 30),
            date(2027, 5, 31),
        ]
        .iter()
        .map(|d| d.at(9, 0, 0, 0))
        .collect();
        assert_eq!(got, want);

        // In a leap year February has its 29th.
        let trigger = Trigger::At {
            at: date(2028, 1, 31).at(9, 0, 0, 0),
        };
        let after = at(date(2028, 2, 1).at(0, 0, 0, 0), &tz);
        let got = fires(&trigger, Some(&monthly), after, &tz, 1);
        assert_eq!(got, vec![date(2028, 2, 29).at(9, 0, 0, 0)]);
    }

    #[test]
    fn the_29th_of_february_falls_on_the_28th_until_the_next_leap_year() {
        let tz = kuala_lumpur();
        let trigger = Trigger::At {
            at: date(2028, 2, 29).at(8, 0, 0, 0),
        };
        let yearly = recurring(1, RecurrenceUnit::Years);
        let after = at(date(2028, 1, 1).at(0, 0, 0, 0), &tz);
        let got = fires(&trigger, Some(&yearly), after, &tz, 5);
        let want: Vec<DateTime> = [
            date(2028, 2, 29),
            date(2029, 2, 28),
            date(2030, 2, 28),
            date(2031, 2, 28),
            date(2032, 2, 29),
        ]
        .iter()
        .map(|d| d.at(8, 0, 0, 0))
        .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn weekly_on_chosen_days_every_other_week() {
        // Starting Thursday 19 November 2026, Mondays and Thursdays, every two
        // weeks. Monday the 16th is before the start and does not fire.
        let tz = kuala_lumpur();
        let trigger = Trigger::At {
            at: date(2026, 11, 19).at(17, 0, 0, 0),
        };
        let recurrence = Recurrence {
            every: 2,
            unit: RecurrenceUnit::Weeks,
            weekdays: vec![Weekday::Thursday, Weekday::Monday],
        };
        let after = at(date(2026, 11, 1).at(0, 0, 0, 0), &tz);
        let got = fires(&trigger, Some(&recurrence), after, &tz, 5);
        let want: Vec<DateTime> = [
            date(2026, 11, 19),
            date(2026, 11, 30),
            date(2026, 12, 3),
            date(2026, 12, 14),
            date(2026, 12, 17),
        ]
        .iter()
        .map(|d| d.at(17, 0, 0, 0))
        .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn weekly_with_no_days_chosen_keeps_the_start_weekday() {
        let tz = kuala_lumpur();
        let trigger = Trigger::At {
            at: date(2026, 11, 19).at(17, 0, 0, 0),
        };
        let weekly = recurring(1, RecurrenceUnit::Weeks);
        let after = at(date(2026, 11, 19).at(18, 0, 0, 0), &tz);
        let got = fires(&trigger, Some(&weekly), after, &tz, 2);
        assert_eq!(
            got,
            vec![
                date(2026, 11, 26).at(17, 0, 0, 0),
                date(2026, 12, 3).at(17, 0, 0, 0)
            ]
        );
    }

    #[test]
    fn every_n_counts_from_the_start_date_even_years_later() {
        let tz = kuala_lumpur();
        // Every 3 days from 1 January 2026. 1 January 2027 is day 365, and
        // 365 = 3 × 121 + 2, so the occurrences around it are days 363 and 366.
        let trigger = Trigger::At {
            at: date(2026, 1, 1).at(6, 0, 0, 0),
        };
        let every_three = recurring(3, RecurrenceUnit::Days);
        let after = at(date(2027, 1, 1).at(0, 0, 0, 0), &tz);
        assert_eq!(
            fires(&trigger, Some(&every_three), after, &tz, 2),
            vec![
                date(2027, 1, 2).at(6, 0, 0, 0),
                date(2027, 1, 5).at(6, 0, 0, 0)
            ]
        );

        // Every 2 months from 31 January: March, May, July — on the 31st.
        let trigger = Trigger::At {
            at: date(2026, 1, 31).at(6, 0, 0, 0),
        };
        let every_two = recurring(2, RecurrenceUnit::Months);
        let after = at(date(2026, 2, 1).at(0, 0, 0, 0), &tz);
        assert_eq!(
            fires(&trigger, Some(&every_two), after, &tz, 3),
            vec![
                date(2026, 3, 31).at(6, 0, 0, 0),
                date(2026, 5, 31).at(6, 0, 0, 0),
                date(2026, 7, 31).at(6, 0, 0, 0),
            ]
        );

        // Every 2 years.
        let trigger = Trigger::At {
            at: date(2026, 6, 15).at(6, 0, 0, 0),
        };
        let biennial = recurring(2, RecurrenceUnit::Years);
        let after = at(date(2026, 6, 16).at(0, 0, 0, 0), &tz);
        assert_eq!(
            fires(&trigger, Some(&biennial), after, &tz, 1),
            vec![date(2028, 6, 15).at(6, 0, 0, 0)]
        );
    }

    #[test]
    fn an_interval_of_zero_never_fires() {
        let tz = kuala_lumpur();
        let trigger = Trigger::At {
            at: date(2026, 12, 25).at(9, 0, 0, 0),
        };
        let never = recurring(0, RecurrenceUnit::Days);
        assert_eq!(
            next_fire(&trigger, Some(&never), Timestamp::UNIX_EPOCH, &tz),
            None
        );
    }

    #[test]
    fn several_offsets_each_fire_once_then_the_time_itself() {
        let tz = kuala_lumpur();
        let offsets = vec![
            Offset {
                amount: 1,
                unit: OffsetUnit::Weeks,
            },
            Offset {
                amount: 2,
                unit: OffsetUnit::Days,
            },
            Offset {
                amount: 1,
                unit: OffsetUnit::Hours,
            },
            Offset {
                amount: 15,
                unit: OffsetUnit::Minutes,
            },
        ];
        let trigger = Trigger::OffsetBefore {
            at: date(2026, 12, 25).at(9, 0, 0, 0),
            offsets: offsets.clone(),
        };
        let mut after = at(date(2026, 12, 1).at(0, 0, 0, 0), &tz);
        let mut seen = Vec::new();
        while let Some(fire) = next_fire(&trigger, None, after, &tz) {
            seen.push((
                fire.at.to_zoned_datetime(tz.clone()).datetime(),
                fire.offset,
            ));
            after = fire.at;
        }
        assert_eq!(
            seen,
            vec![
                (date(2026, 12, 18).at(9, 0, 0, 0), Some(offsets[0])),
                (date(2026, 12, 23).at(9, 0, 0, 0), Some(offsets[1])),
                (date(2026, 12, 25).at(8, 0, 0, 0), Some(offsets[2])),
                (date(2026, 12, 25).at(8, 45, 0, 0), Some(offsets[3])),
                (date(2026, 12, 25).at(9, 0, 0, 0), None),
            ]
        );
    }

    #[test]
    fn a_day_offset_is_a_calendar_day_across_a_dst_change() {
        // 1 day before 07:00 on Sunday 8 March 2026 is 07:00 on the Saturday:
        // New York's clocks went forward at 02:00 in between, so the two are
        // 23 hours apart, not 24.
        let tz = new_york();
        let trigger = Trigger::OffsetBefore {
            at: date(2026, 3, 8).at(7, 0, 0, 0),
            offsets: vec![Offset {
                amount: 1,
                unit: OffsetUnit::Days,
            }],
        };
        let after = at(date(2026, 3, 1).at(0, 0, 0, 0), &tz);
        let fire = next_fire(&trigger, None, after, &tz).expect("fires");
        assert_eq!(
            fire.at.to_zoned_datetime(tz.clone()).datetime(),
            date(2026, 3, 7).at(7, 0, 0, 0)
        );
        assert_eq!(fire.occurrence - fire.at, SignedDuration::from_hours(23));
    }

    #[test]
    fn a_recurring_reminder_with_an_offset_alternates_with_its_lead() {
        let tz = kuala_lumpur();
        let trigger = Trigger::OffsetBefore {
            at: date(2026, 11, 20).at(9, 0, 0, 0),
            offsets: vec![Offset {
                amount: 1,
                unit: OffsetUnit::Hours,
            }],
        };
        let daily = recurring(1, RecurrenceUnit::Days);
        let after = at(date(2026, 11, 20).at(0, 0, 0, 0), &tz);
        assert_eq!(
            fires(&trigger, Some(&daily), after, &tz, 4),
            vec![
                date(2026, 11, 20).at(8, 0, 0, 0),
                date(2026, 11, 20).at(9, 0, 0, 0),
                date(2026, 11, 21).at(8, 0, 0, 0),
                date(2026, 11, 21).at(9, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn a_lead_that_reaches_past_later_occurrences_still_comes_out_in_order() {
        // Daily at 09:00 with a 2-day lead: the lead of the day-after-tomorrow
        // fires at the same 09:00 as today's own fire, and they are one fire.
        let tz = kuala_lumpur();
        let trigger = Trigger::OffsetBefore {
            at: date(2026, 11, 20).at(9, 0, 0, 0),
            offsets: vec![Offset {
                amount: 2,
                unit: OffsetUnit::Days,
            }],
        };
        let daily = recurring(1, RecurrenceUnit::Days);
        let after = at(date(2026, 11, 25).at(10, 0, 0, 0), &tz);
        let fire = next_fire(&trigger, Some(&daily), after, &tz).expect("fires");
        assert_eq!(
            fire.at.to_zoned_datetime(tz.clone()).datetime(),
            date(2026, 11, 26).at(9, 0, 0, 0)
        );
        assert_eq!(fire.offset, None, "the occurrence's own fire wins the tie");
        let next = next_fire(&trigger, Some(&daily), fire.at, &tz).expect("fires");
        assert_eq!(
            next.at.to_zoned_datetime(tz.clone()).datetime(),
            date(2026, 11, 27).at(9, 0, 0, 0)
        );
    }
}
