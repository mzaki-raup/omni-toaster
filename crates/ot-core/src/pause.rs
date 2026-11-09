//! Pausing all notifications for a while.
//!
//! One click from the tray stops toasts and announcements for half an hour, an
//! hour, or until the user says otherwise (`docs/specs/tray-and-settings.md`
//! §4). **Azan is never suppressed by a pause** — a user who wants that turns
//! Azan off in Settings ▸ Prayer, which is a deliberate choice rather than a
//! side effect of silencing everything for a meeting.
//!
//! This holds the state and the arithmetic only. Who obeys it is the
//! scheduler's business, and how it reads is the window's: a paused app must
//! say so in the tray tooltip and in the settings status line, because the
//! worst outcome here is a user who has forgotten they muted it.

use chrono::{DateTime, Duration, Local};

/// The windows the tray menu offers. Fixed by the spec, not a free number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseFor {
    ThirtyMinutes,
    OneHour,
    UntilResumed,
}

impl PauseFor {
    /// The Fluent key naming this window in the menu.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::ThirtyMinutes => "tray-pause-30",
            Self::OneHour => "tray-pause-60",
            Self::UntilResumed => "tray-pause-until-resumed",
        }
    }

    /// How long it lasts, or `None` for "until the user resumes".
    pub fn duration(self) -> Option<Duration> {
        match self {
            Self::ThirtyMinutes => Some(Duration::minutes(30)),
            Self::OneHour => Some(Duration::hours(1)),
            Self::UntilResumed => None,
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::ThirtyMinutes, Self::OneHour, Self::UntilResumed]
    }
}

/// Whether notifications are paused, and until when.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pause {
    /// Notifications are being delivered.
    #[default]
    Running,
    /// Paused until this moment, after which it lapses on its own.
    Until(DateTime<Local>),
    /// Paused with no end: only the user resumes it.
    UntilResumed,
}

impl Pause {
    /// Begin a pause from `now`.
    ///
    /// `now` is passed in rather than read here, so the arithmetic can be
    /// tested at the times that matter — a pause started at 23:50 ends
    /// tomorrow, and nothing about that may depend on what the clock says
    /// while the test runs.
    pub fn start(window: PauseFor, now: DateTime<Local>) -> Self {
        match window.duration() {
            Some(duration) => Self::Until(now + duration),
            None => Self::UntilResumed,
        }
    }

    /// Whether notifications are suppressed at `now`.
    ///
    /// A pause whose moment has passed is simply not active: it lapses rather
    /// than needing anyone to clear it, so a missed tick cannot leave the app
    /// silent for ever.
    pub fn is_active(&self, now: DateTime<Local>) -> bool {
        match self {
            Self::Running => false,
            Self::UntilResumed => true,
            Self::Until(moment) => now < *moment,
        }
    }

    /// When it lapses, for the tooltip and the status line. `None` means it
    /// does not lapse on its own.
    pub fn expires_at(&self) -> Option<DateTime<Local>> {
        match self {
            Self::Until(moment) => Some(*moment),
            Self::Running | Self::UntilResumed => None,
        }
    }

    /// Drop a pause that has lapsed, so the state does not keep claiming a
    /// moment in the past. Returns whether anything changed.
    pub fn lapse(&mut self, now: DateTime<Local>) -> bool {
        if matches!(self, Self::Until(_)) && !self.is_active(now) {
            *self = Self::Running;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(hour: u32, minute: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 11, 9, hour, minute, 0)
            .single()
            .expect("a real local time")
    }

    #[test]
    fn running_is_the_default_and_suppresses_nothing() {
        assert_eq!(Pause::default(), Pause::Running);
        assert!(!Pause::default().is_active(at(12, 0)));
        assert_eq!(Pause::default().expires_at(), None);
    }

    #[test]
    fn the_two_timed_windows_are_the_ones_the_spec_fixes() {
        let noon = at(12, 0);
        assert_eq!(
            Pause::start(PauseFor::ThirtyMinutes, noon).expires_at(),
            Some(at(12, 30))
        );
        assert_eq!(
            Pause::start(PauseFor::OneHour, noon).expires_at(),
            Some(at(13, 0))
        );
    }

    #[test]
    fn a_pause_started_before_midnight_ends_the_next_day() {
        // The case the ledger calls out: naive arithmetic on the hour field
        // would wrap to 00:20 *today*, unpausing half an hour before it
        // should and on the wrong date.
        let late = at(23, 50);
        let pause = Pause::start(PauseFor::ThirtyMinutes, late);
        let end = pause.expires_at().expect("a timed pause has an end");
        assert_eq!(end, late + Duration::minutes(30));
        assert_eq!(
            end.date_naive(),
            late.date_naive().succ_opt().expect("tomorrow")
        );
        assert!(pause.is_active(at(23, 59)), "still paused before midnight");
        assert!(
            pause.is_active(end - Duration::seconds(1)),
            "still paused a second before it ends, which is after midnight"
        );
        assert!(!pause.is_active(end), "not paused at the moment it ends");
    }

    #[test]
    fn until_resumed_never_lapses() {
        let pause = Pause::start(PauseFor::UntilResumed, at(12, 0));
        assert_eq!(pause, Pause::UntilResumed);
        assert_eq!(pause.expires_at(), None);
        // Any time at all, including days later.
        assert!(pause.is_active(at(12, 0)));
        assert!(pause.is_active(at(23, 59) + Duration::days(400)));
    }

    #[test]
    fn a_lapsed_pause_clears_itself_but_an_open_one_does_not() {
        let mut timed = Pause::start(PauseFor::OneHour, at(12, 0));
        assert!(!timed.lapse(at(12, 30)), "not due yet");
        assert_eq!(timed, Pause::Until(at(13, 0)));
        assert!(timed.lapse(at(13, 0)), "due, so it clears");
        assert_eq!(timed, Pause::Running);
        assert!(!timed.lapse(at(14, 0)), "already running");

        let mut open = Pause::UntilResumed;
        assert!(!open.lapse(at(23, 59) + Duration::days(7)));
        assert_eq!(open, Pause::UntilResumed, "only the user resumes this one");
    }

    #[test]
    fn every_window_names_a_key_and_they_are_all_different() {
        let keys: Vec<&str> = PauseFor::all().iter().map(|w| w.name_key()).collect();
        let mut unique = keys.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            keys.len(),
            "two windows share a key: {keys:?}"
        );
    }
}
