//! When a fatigue reminder fires (`docs/specs/reminders.md` §3): counted from
//! the start of the working day, not from a date.
//!
//! An eye break, a posture check or a drink comes every so many minutes after
//! the day began; the work-hour limit comes once, so many minutes after it.
//! The working day itself — when it starts, and that an hour away from the PC
//! ends it — is the scheduler's (`scheduler::Core`); this is the pure part,
//! given that start.

use jiff::{SignedDuration, Timestamp};
use ot_store::reminders::Trigger;

use crate::recurrence::Fire;

/// The first fire strictly after `after`, for a working day that began at
/// `start`, or `None` for a trigger that is not a fatigue schedule, or a
/// work-hour limit already behind.
pub fn next_fire(trigger: &Trigger, start: Timestamp, after: Timestamp) -> Option<Fire> {
    let at = match trigger {
        Trigger::Every { minutes } => {
            let step = i64::from(*minutes) * 60;
            if step == 0 {
                return None;
            }
            let elapsed = after.duration_since(start).as_secs();
            // The first multiple of the step strictly after `after`, and
            // never the start itself: the first break comes one step in.
            let steps = if elapsed < 0 { 1 } else { elapsed / step + 1 };
            start
                .checked_add(SignedDuration::from_secs(steps.checked_mul(step)?))
                .ok()?
        }
        Trigger::WorkHours { minutes } => {
            let at = start
                .checked_add(SignedDuration::from_mins(i64::from(*minutes)))
                .ok()?;
            if at <= after {
                return None;
            }
            at
        }
        Trigger::At { .. } | Trigger::OffsetBefore { .. } => return None,
    };
    Some(Fire {
        at,
        occurrence: at,
        offset: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ot_store::reminders::FATIGUE;

    fn instant(text: &str) -> Timestamp {
        text.parse().expect("an instant")
    }

    fn fires(trigger: &Trigger, start: &str, until: &str) -> Vec<Timestamp> {
        let start = instant(start);
        let until = instant(until);
        let mut after = start;
        let mut out = Vec::new();
        while let Some(fire) = next_fire(trigger, start, after) {
            if fire.at > until {
                break;
            }
            out.push(fire.at);
            after = fire.at;
        }
        out
    }

    #[test]
    fn each_kind_keeps_its_default_schedule() {
        let trigger = |key: &str| {
            let (_, repeats, minutes) = FATIGUE
                .iter()
                .find(|(k, _, _)| *k == key)
                .copied()
                .expect("a fatigue kind");
            if repeats {
                Trigger::Every { minutes }
            } else {
                Trigger::WorkHours { minutes }
            }
        };
        let day = ("2026-11-20T01:00:00Z", "2026-11-20T03:00:00Z");
        assert_eq!(
            fires(&trigger("reminder-fatigue-eye"), day.0, day.1).len(),
            6,
            "every 20 minutes over two hours"
        );
        assert_eq!(
            fires(&trigger("reminder-fatigue-posture"), day.0, day.1),
            vec![
                instant("2026-11-20T01:30:00Z"),
                instant("2026-11-20T02:00:00Z"),
                instant("2026-11-20T02:30:00Z"),
                instant("2026-11-20T03:00:00Z"),
            ]
        );
        assert_eq!(
            fires(&trigger("reminder-fatigue-hydration"), day.0, day.1),
            vec![
                instant("2026-11-20T02:00:00Z"),
                instant("2026-11-20T03:00:00Z")
            ]
        );
        assert_eq!(
            fires(
                &trigger("reminder-fatigue-work-hours"),
                day.0,
                "2026-11-21T01:00:00Z"
            ),
            vec![instant("2026-11-20T09:00:00Z")],
            "once, eight hours in"
        );
    }

    #[test]
    fn the_next_break_is_strictly_after_and_never_at_the_start() {
        let every = Trigger::Every { minutes: 20 };
        let start = instant("2026-11-20T01:00:00Z");
        let next = |after| next_fire(&every, start, instant(after)).map(|f| f.at);
        assert_eq!(
            next("2026-11-20T01:00:00Z"),
            Some(instant("2026-11-20T01:20:00Z"))
        );
        assert_eq!(
            next("2026-11-20T01:20:00Z"),
            Some(instant("2026-11-20T01:40:00Z"))
        );
        assert_eq!(
            next("2026-11-20T01:39:59Z"),
            Some(instant("2026-11-20T01:40:00Z"))
        );
        assert_eq!(
            next("2026-11-20T00:00:00Z"),
            Some(instant("2026-11-20T01:20:00Z"))
        );
        assert_eq!(
            next_fire(&Trigger::Every { minutes: 0 }, start, start),
            None
        );
    }
}
