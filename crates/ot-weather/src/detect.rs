//! Deciding when a change is worth a toast (FEAT-0068, `docs/specs/weather.md`
//! §4 and §7).
//!
//! The goal is a toast when the weather **changes**, not one per poll. [`Detector`]
//! is a plain value fed one fresh [`Observation`] at a time; it holds no clock,
//! does no I/O, and says what changed.
//!
//! - **A condition change fires once**, and only when the new class is seen in two
//!   consecutive *distinct* readings. The class that counts as "before" is the
//!   last one **notified**, not the last one seen, so a class that flaps and
//!   returns never reaches the user. A reading with the same `at` as the one
//!   before it (a poll faster than the service updates) is not a second sighting.
//! - **The first reading is a baseline**, silent: a toast for the weather the
//!   moment the app starts would be noise.
//! - **A temperature threshold** fires when the move over a window reaches the
//!   delta — exactly the delta fires, less does not — and then not again until the
//!   move has fallen back under it, so a long cold front is one toast.
//! - **Stale data changes nothing.** A cached answer is shown, never compared, so
//!   a change is only ever derived from a fresh response.
//! - A class the app does not know ([`Condition::Unknown`]) is no information: it
//!   neither confirms nor breaks a change, and is never announced.

use jiff::{SignedDuration, Timestamp};

use crate::condition::{Condition, classify};
use crate::observation::Observation;

/// Which way a temperature moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Fall,
    Rise,
}

/// A temperature move worth telling about: `delta_c` degrees or more in the
/// direction, within `window`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Threshold {
    pub direction: Direction,
    pub delta_c: f64,
    pub window: SignedDuration,
}

impl Threshold {
    /// The spec's example: a drop of 5 °C within 3 hours.
    pub fn default_fall() -> Self {
        Self {
            direction: Direction::Fall,
            delta_c: 5.0,
            window: SignedDuration::from_hours(3),
        }
    }
}

/// What the detector found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Change {
    /// The class changed from the last one notified.
    Condition { from: Condition, to: Condition },
    /// The temperature moved by `delta_c` (always positive) in `direction`,
    /// from `from_c` to `to_c`, within the threshold's window.
    Temperature {
        direction: Direction,
        delta_c: f64,
        from_c: f64,
        to_c: f64,
    },
}

/// The memory a detector needs between readings.
#[derive(Debug, Clone, Default)]
pub struct Detector {
    /// The last class told to the user, or the baseline.
    notified: Option<Condition>,
    /// A different class seen once, waiting for a second sighting.
    pending: Option<Condition>,
    /// The `at` of the last reading used.
    last_at: Option<Timestamp>,
    /// Whether a threshold has fired and not yet been left behind: fall, rise.
    fired: [bool; 2],
}

impl Detector {
    /// A detector that has seen nothing: its first reading is a baseline.
    pub fn new() -> Self {
        Self::default()
    }

    /// A detector that already told the user about `notified`, as after a restart.
    pub fn starting_from(notified: Option<Condition>) -> Self {
        Self {
            notified,
            ..Self::default()
        }
    }

    /// The class the user was last told, for keeping across a restart.
    pub fn notified(&self) -> Option<Condition> {
        self.notified
    }

    /// Feed one reading. `fresh` is whether it came from a fetch just now; a
    /// cached answer is never compared, so it changes nothing.
    pub fn observe(
        &mut self,
        reading: &Observation,
        fresh: bool,
        thresholds: &[Threshold],
    ) -> Vec<Change> {
        if !fresh {
            return Vec::new();
        }
        // The same reading twice is one reading: the service moves in
        // 15-minute steps and a poll can arrive faster.
        if self.last_at.is_some_and(|last| reading.at <= last) {
            return Vec::new();
        }
        self.last_at = Some(reading.at);

        let mut changes = Vec::new();
        if let Some(change) = self.condition(classify(reading.code)) {
            changes.push(change);
        }
        for threshold in thresholds {
            if let Some(change) = self.temperature(reading, threshold) {
                changes.push(change);
            }
        }
        changes
    }

    fn condition(&mut self, class: Condition) -> Option<Change> {
        if class == Condition::Unknown {
            return None;
        }
        let Some(notified) = self.notified else {
            self.notified = Some(class);
            return None;
        };
        if class == notified {
            self.pending = None;
            return None;
        }
        if self.pending == Some(class) {
            self.pending = None;
            self.notified = Some(class);
            return Some(Change::Condition {
                from: notified,
                to: class,
            });
        }
        self.pending = Some(class);
        None
    }

    fn temperature(&mut self, reading: &Observation, threshold: &Threshold) -> Option<Change> {
        let slot = match threshold.direction {
            Direction::Fall => 0,
            Direction::Rise => 1,
        };
        let earliest = reading.at - threshold.window;
        // The biggest move toward the reading from any point inside the window.
        let from_c = reading
            .hourly
            .iter()
            .filter(|(at, _)| *at >= earliest && *at <= reading.at)
            .map(|(_, degrees)| *degrees)
            .reduce(|best, degrees| match threshold.direction {
                Direction::Fall => best.max(degrees),
                Direction::Rise => best.min(degrees),
            })?;
        let moved = match threshold.direction {
            Direction::Fall => from_c - reading.temperature_c,
            Direction::Rise => reading.temperature_c - from_c,
        };
        if moved < threshold.delta_c {
            self.fired[slot] = false;
            return None;
        }
        if self.fired[slot] {
            return None;
        }
        self.fired[slot] = true;
        Some(Change::Temperature {
            direction: threshold.direction,
            delta_c: moved,
            from_c,
            to_c: reading.temperature_c,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(minutes_after_noon: i64) -> Timestamp {
        "2026-11-26T12:00:00Z".parse::<Timestamp>().expect("t")
            + SignedDuration::from_mins(minutes_after_noon)
    }

    /// A reading at `minutes` past noon with one code and a flat temperature.
    fn reading(minutes: i64, code: u8) -> Observation {
        Observation {
            at: at(minutes),
            temperature_c: 25.0,
            code,
            hourly: Vec::new(),
        }
    }

    const CLEAR: u8 = 0;
    const RAIN: u8 = 61;
    const STORM: u8 = 95;
    const FOG: u8 = 45;

    fn feed(detector: &mut Detector, codes: &[u8]) -> Vec<Vec<Change>> {
        codes
            .iter()
            .enumerate()
            .map(|(i, code)| detector.observe(&reading(i as i64 * 15, *code), true, &[]))
            .collect()
    }

    fn fired(all: &[Vec<Change>]) -> Vec<usize> {
        all.iter()
            .enumerate()
            .filter(|(_, changes)| !changes.is_empty())
            .map(|(i, _)| i)
            .collect()
    }

    #[test]
    fn the_first_reading_is_a_baseline_and_says_nothing() {
        let mut detector = Detector::new();
        assert!(feed(&mut detector, &[RAIN]).iter().all(Vec::is_empty));
        assert_eq!(detector.notified(), Some(Condition::Rain));
    }

    #[test]
    fn a_change_fires_once_on_its_second_sighting_and_not_per_poll() {
        let mut detector = Detector::new();
        let all = feed(&mut detector, &[CLEAR, RAIN, RAIN, RAIN, RAIN]);
        assert_eq!(fired(&all), [2], "only the second rain");
        assert_eq!(
            all[2],
            [Change::Condition {
                from: Condition::Clear,
                to: Condition::Rain
            }]
        );
    }

    #[test]
    fn flapping_between_two_classes_never_notifies() {
        let mut detector = Detector::new();
        let all = feed(
            &mut detector,
            &[CLEAR, RAIN, CLEAR, RAIN, CLEAR, RAIN, CLEAR],
        );
        assert!(fired(&all).is_empty(), "{all:?}");
        assert_eq!(detector.notified(), Some(Condition::Clear));
    }

    #[test]
    fn three_different_classes_in_a_row_confirm_none_of_them() {
        let mut detector = Detector::new();
        let all = feed(&mut detector, &[CLEAR, RAIN, STORM, FOG, STORM, STORM]);
        assert_eq!(fired(&all), [5], "only the second storm in a row");
    }

    #[test]
    fn the_comparison_is_with_the_last_notified_class_not_the_last_seen() {
        let mut detector = Detector::new();
        // Rain is notified at index 2. A clear then a rain after that is the
        // notified class again, which is nothing to tell; clear twice in a row is.
        let all = feed(
            &mut detector,
            &[CLEAR, RAIN, RAIN, CLEAR, RAIN, RAIN, CLEAR, CLEAR],
        );
        assert_eq!(fired(&all), [2, 7]);
        assert_eq!(
            all[7],
            [Change::Condition {
                from: Condition::Rain,
                to: Condition::Clear
            }]
        );
    }

    #[test]
    fn a_class_that_returns_to_the_notified_one_resets_the_wait() {
        let mut detector = Detector::starting_from(Some(Condition::Clear));
        // Rain once, back to clear, rain once: never twice in a row.
        let all = feed(&mut detector, &[RAIN, CLEAR, RAIN, CLEAR]);
        assert!(fired(&all).is_empty());
    }

    #[test]
    fn a_reading_repeated_is_not_a_second_sighting() {
        let mut detector = Detector::starting_from(Some(Condition::Clear));
        assert!(detector.observe(&reading(0, RAIN), true, &[]).is_empty());
        // The same moment again, as a poll faster than the service updates.
        assert!(detector.observe(&reading(0, RAIN), true, &[]).is_empty());
        // A later one is.
        assert_eq!(detector.observe(&reading(15, RAIN), true, &[]).len(), 1);
    }

    #[test]
    fn stale_data_changes_nothing_and_counts_for_nothing() {
        let mut detector = Detector::starting_from(Some(Condition::Clear));
        for i in 0..5 {
            assert!(
                detector
                    .observe(&reading(i * 15, RAIN), false, &[])
                    .is_empty()
            );
        }
        assert_eq!(detector.notified(), Some(Condition::Clear));
        // The stale readings were not a first sighting: one fresh rain is still one.
        assert!(detector.observe(&reading(90, RAIN), true, &[]).is_empty());
        assert_eq!(detector.observe(&reading(105, RAIN), true, &[]).len(), 1);
    }

    #[test]
    fn an_unknown_code_is_no_information() {
        let mut detector = Detector::starting_from(Some(Condition::Clear));
        let all = feed(&mut detector, &[RAIN, 250, RAIN]);
        assert_eq!(
            fired(&all),
            [2],
            "an unknown code between two rains neither breaks nor confirms"
        );
        // And an unknown code is never the baseline.
        let mut fresh = Detector::new();
        assert!(fresh.observe(&reading(0, 250), true, &[]).is_empty());
        assert_eq!(fresh.notified(), None);
    }

    /// A reading at noon with these hourly temperatures at 3, 2, 1 and 0 hours before.
    fn with_history(now_c: f64, history: &[(i64, f64)]) -> Observation {
        Observation {
            at: at(0),
            temperature_c: now_c,
            code: CLEAR,
            hourly: history
                .iter()
                .map(|(minutes_before, degrees)| (at(-minutes_before), *degrees))
                .collect(),
        }
    }

    #[test]
    fn the_threshold_fires_at_the_boundary_and_not_inside_it() {
        let fall = [Threshold::default_fall()];
        // 30 now, 35 three hours ago: exactly 5 down, in the window's first minute.
        let mut at_boundary = Detector::new();
        let changes = at_boundary.observe(&with_history(30.0, &[(180, 35.0)]), true, &fall);
        assert_eq!(
            changes,
            [Change::Temperature {
                direction: Direction::Fall,
                delta_c: 5.0,
                from_c: 35.0,
                to_c: 30.0
            }]
        );
        // 4.9 is inside the boundary.
        let mut inside = Detector::new();
        assert!(
            inside
                .observe(&with_history(30.1, &[(180, 35.0)]), true, &fall)
                .is_empty()
        );
    }

    #[test]
    fn a_move_from_before_the_window_does_not_count() {
        let fall = [Threshold::default_fall()];
        let mut detector = Detector::new();
        // 3 hours and 1 minute ago: just outside.
        assert!(
            detector
                .observe(&with_history(30.0, &[(181, 36.0)]), true, &fall)
                .is_empty()
        );
    }

    #[test]
    fn the_biggest_move_in_the_window_is_the_one_reported() {
        let fall = [Threshold::default_fall()];
        let mut detector = Detector::new();
        let changes = detector.observe(
            &with_history(24.0, &[(180, 29.5), (120, 31.0), (60, 27.0)]),
            true,
            &fall,
        );
        assert!(matches!(
            changes.as_slice(),
            [Change::Temperature { from_c, delta_c, .. }] if *from_c == 31.0 && *delta_c == 7.0
        ));
    }

    #[test]
    fn a_rise_is_its_own_threshold_and_a_fall_does_not_trip_it() {
        let rise = [Threshold {
            direction: Direction::Rise,
            delta_c: 6.0,
            window: SignedDuration::from_hours(2),
        }];
        let mut detector = Detector::new();
        assert!(
            detector
                .observe(&with_history(20.0, &[(60, 26.0)]), true, &rise)
                .is_empty(),
            "a fall of 6 is not a rise"
        );
        let mut detector = Detector::new();
        assert_eq!(
            detector
                .observe(&with_history(32.0, &[(60, 26.0)]), true, &rise)
                .len(),
            1
        );
    }

    #[test]
    fn a_long_fall_is_one_toast_and_a_second_fall_after_recovering_is_another() {
        let fall = [Threshold::default_fall()];
        let mut detector = Detector::new();
        let reading_at = |minutes: i64, now_c: f64, then_c: f64| Observation {
            at: at(minutes),
            temperature_c: now_c,
            code: CLEAR,
            hourly: vec![(at(minutes - 120), then_c)],
        };
        assert_eq!(
            detector
                .observe(&reading_at(0, 25.0, 31.0), true, &fall)
                .len(),
            1
        );
        // Still down by 6 at the next poll: the same cold front, not a new one.
        assert!(
            detector
                .observe(&reading_at(15, 24.5, 30.5), true, &fall)
                .is_empty()
        );
        assert!(
            detector
                .observe(&reading_at(30, 24.0, 30.0), true, &fall)
                .is_empty()
        );
        // It levels off, which re-arms it.
        assert!(
            detector
                .observe(&reading_at(45, 24.0, 25.0), true, &fall)
                .is_empty()
        );
        // Another drop is another toast.
        assert_eq!(
            detector
                .observe(&reading_at(60, 19.0, 25.0), true, &fall)
                .len(),
            1
        );
    }

    #[test]
    fn stale_data_never_trips_a_threshold() {
        let fall = [Threshold::default_fall()];
        let mut detector = Detector::new();
        assert!(
            detector
                .observe(&with_history(20.0, &[(60, 35.0)]), false, &fall)
                .is_empty()
        );
        // And it did not arm or disarm anything: the fresh one still fires.
        assert_eq!(
            detector
                .observe(&with_history(20.0, &[(60, 35.0)]), true, &fall)
                .len(),
            1
        );
    }

    #[test]
    fn a_reading_with_no_hours_has_no_threshold_to_trip() {
        let mut detector = Detector::new();
        assert!(
            detector
                .observe(&reading(0, CLEAR), true, &[Threshold::default_fall()])
                .is_empty()
        );
    }

    #[test]
    fn a_condition_and_a_temperature_change_in_one_reading_are_both_reported() {
        let fall = [Threshold::default_fall()];
        let mut detector = Detector::starting_from(Some(Condition::Clear));
        let storm_cold = |minutes: i64| Observation {
            at: at(minutes),
            temperature_c: 22.0,
            code: STORM,
            hourly: vec![(at(minutes - 60), 30.0)],
        };
        assert_eq!(
            detector.observe(&storm_cold(0), true, &fall).len(),
            1,
            "the fall, and the storm once"
        );
        let again = detector.observe(&storm_cold(15), true, &fall);
        assert!(matches!(
            again.as_slice(),
            [Change::Condition {
                to: Condition::Storm,
                ..
            }]
        ));
    }
}
