//! The scheduler thread: the single source of time (`docs/specs/reminders.md`
//! §7, §12).
//!
//! One named thread owns a queue of every active reminder's next fire time,
//! ordered by that time. It waits on its command channel with a bounded
//! timeout — never longer than [`MAX_WAIT`] — and re-reads the wall clock on
//! every wake, so a machine that slept cannot leave it holding a stale
//! deadline.
//!
//! The queue is rebuilt from the database, not trusted, after a clock change,
//! a time-zone change, a resume and at local midnight. The clock change and
//! the resume come from Windows through `ot_platform::system_events` as
//! [`Command::Recompute`]; the thread also notices a clock jump on its own,
//! by comparing the wall clock against the monotonic clock across each wait,
//! and re-reads the zone on every wake.
//!
//! Fires go to the UI over a bounded channel. A full channel holds the fire
//! and retries; a fire is never dropped.
//!
//! [`Core`] is the queue and every rule about it, driven with an explicit
//! `now` and zone so the tests pin the time. [`Scheduler`] is the thread
//! around it.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TrySendError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use ot_store::Store;
use ot_store::reminders::{Reminder, Status};

use crate::recurrence::{Fire, next_fire};

/// The longest the thread waits before looking at the clock again, however
/// far off the next fire is.
pub const MAX_WAIT: Duration = Duration::from_secs(30);

/// How many fires the UI may be behind before the scheduler holds the rest.
pub const FIRE_CAPACITY: usize = 64;

/// How soon a held fire is offered to the UI again.
const HELD_RETRY: Duration = Duration::from_millis(100);

/// How far the wall clock may drift from the monotonic clock across one wait
/// before it counts as the clock having been changed.
const JUMP_TOLERANCE: SignedDuration = SignedDuration::from_secs(2);

/// Why the queue is being rebuilt. Logged, so the log says why a fire time
/// moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recompute {
    Midnight,
    ClockChanged,
    ZoneChanged,
    Resumed,
}

/// What the rest of the app tells the scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// A reminder was created or edited: read it again and requeue it.
    Changed(i64),
    /// A reminder was deleted.
    Removed(i64),
    /// Throw the queue away and rebuild it from the database.
    Recompute(Recompute),
}

/// One fire, sent to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fired {
    pub reminder_id: i64,
    pub fire: Fire,
}

/// The UI's receiving end has gone, so there is no one left to fire to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disconnected;

/// The bounded channel fires travel on, scheduler to UI.
pub fn fire_channel() -> (SyncSender<Fired>, Receiver<Fired>) {
    mpsc::sync_channel(FIRE_CAPACITY)
}

/// The queue and the rules about it, with no clock and no thread of its own.
#[derive(Debug)]
pub struct Core {
    store: Store,
    tz: TimeZone,
    /// Ordered by fire time; the id breaks ties.
    queue: BTreeSet<(Timestamp, i64)>,
    /// Each queued reminder's fire, by id.
    pending: BTreeMap<i64, Fire>,
    /// Fires the UI's channel had no room for, oldest first.
    held: VecDeque<Fired>,
    /// Everything up to this instant has been fired. A rebuild starts from
    /// here, so a clock jumped forward fires what it jumped over, once.
    horizon: Timestamp,
    /// The next local midnight, when the queue is rebuilt.
    midnight: Timestamp,
}

impl Core {
    /// Build the queue from every active reminder, as of `now`.
    pub fn new(store: Store, now: Timestamp, tz: TimeZone) -> Self {
        let midnight = next_midnight(now, &tz);
        let mut core = Self {
            store,
            tz,
            queue: BTreeSet::new(),
            pending: BTreeMap::new(),
            held: VecDeque::new(),
            horizon: now,
            midnight,
        };
        core.rebuild(now);
        core
    }

    /// The next fire waiting, if any.
    pub fn next(&self) -> Option<(i64, Fire)> {
        let &(_, id) = self.queue.first()?;
        self.pending.get(&id).map(|fire| (id, *fire))
    }

    /// Fires waiting for room in the UI's channel.
    pub fn held(&self) -> usize {
        self.held.len()
    }

    /// Act on a command.
    pub fn apply(&mut self, command: Command, now: Timestamp) {
        match command {
            Command::Changed(id) => match self.store.reminder(id) {
                Ok(Some(reminder)) => self.schedule(&reminder, now),
                Ok(None) => self.unschedule(id),
                Err(error) => {
                    tracing::warn!(%error, reminder = id, "a changed reminder could not be read");
                    self.unschedule(id);
                }
            },
            Command::Removed(id) => self.unschedule(id),
            Command::Recompute(why) => {
                tracing::info!(?why, "rebuilding the reminder queue");
                self.rebuild(now);
            }
        }
    }

    /// Follow a new time zone: every wall-clock time means a different
    /// instant now, so the queue is rebuilt.
    pub fn set_zone(&mut self, tz: TimeZone, now: Timestamp) {
        if tz != self.tz {
            self.tz = tz;
            self.apply(Command::Recompute(Recompute::ZoneChanged), now);
        }
    }

    /// Fire everything due at `now`, and rebuild first if midnight passed.
    pub fn tick(&mut self, now: Timestamp, out: &SyncSender<Fired>) -> Result<(), Disconnected> {
        if now >= self.midnight {
            self.apply(Command::Recompute(Recompute::Midnight), now);
        }

        while let Some(&(at, id)) = self.queue.first() {
            if at > now {
                break;
            }
            self.queue.pop_first();
            let Some(fire) = self.pending.remove(&id) else {
                continue;
            };
            self.held.push_back(Fired {
                reminder_id: id,
                fire,
            });
            // The next fire counts from now, not from the one just taken, so
            // a clock jumped over three days fires a daily reminder once, not
            // three times (catching up properly is docs/specs/reminders.md
            // §6, FEAT-0045).
            self.requeue_after_fire(id, fire.at.max(now));
        }

        self.horizon = self.horizon.max(now);
        self.flush(now, out)
    }

    /// How long to wait before the next look at the clock.
    pub fn wait(&self, now: Timestamp) -> Duration {
        if !self.held.is_empty() {
            return HELD_RETRY;
        }
        let mut until = self.midnight;
        if let Some(&(at, _)) = self.queue.first() {
            until = until.min(at);
        }
        Duration::try_from(until.duration_since(now))
            .unwrap_or(Duration::ZERO)
            .min(MAX_WAIT)
    }

    /// Offer held fires to the UI, oldest first, until its channel is full.
    fn flush(&mut self, now: Timestamp, out: &SyncSender<Fired>) -> Result<(), Disconnected> {
        while let Some(fired) = self.held.pop_front() {
            match out.try_send(fired) {
                Ok(()) => {
                    // The line the timing check reads: the target and how far
                    // past it the fire left (docs/specs/reminders.md §7).
                    let late_ms = now.duration_since(fired.fire.at).as_millis();
                    tracing::info!(
                        reminder = fired.reminder_id,
                        target = %fired.fire.at,
                        late_ms,
                        "reminder fired"
                    );
                }
                Err(TrySendError::Full(fired)) => {
                    self.held.push_front(fired);
                    tracing::debug!(held = self.held.len(), "the UI is behind; holding fires");
                    break;
                }
                Err(TrySendError::Disconnected(_)) => return Err(Disconnected),
            }
        }
        Ok(())
    }

    /// Rebuild the queue from the database. Starts from the horizon, or from
    /// `now` if the clock went back past it.
    fn rebuild(&mut self, now: Timestamp) {
        self.queue.clear();
        self.pending.clear();
        let after = self.horizon.min(now);
        self.horizon = after;
        match self.store.reminders() {
            Ok(reminders) => {
                for reminder in &reminders {
                    self.schedule(reminder, after);
                }
            }
            Err(error) => tracing::warn!(%error, "the reminders could not be read"),
        }
        self.midnight = next_midnight(now, &self.tz);
    }

    /// Queue a reminder's first fire after `after`, if it is active and has
    /// one.
    fn schedule(&mut self, reminder: &Reminder, after: Timestamp) {
        self.unschedule(reminder.id);
        if reminder.status != Status::Active {
            return;
        }
        let next = next_fire(
            &reminder.draft.trigger,
            reminder.draft.recurrence.as_ref(),
            after,
            &self.tz,
        );
        self.queue_fire(reminder.id, next);
    }

    /// After a fire: queue the next one, or mark the reminder done if that
    /// was its last, so a clock set back cannot fire it a second time.
    fn requeue_after_fire(&mut self, id: i64, after: Timestamp) {
        let reminder = match self.store.reminder(id) {
            Ok(Some(reminder)) if reminder.status == Status::Active => reminder,
            Ok(_) => return,
            Err(error) => {
                tracing::warn!(%error, reminder = id, "a fired reminder could not be read again");
                return;
            }
        };
        let next = next_fire(
            &reminder.draft.trigger,
            reminder.draft.recurrence.as_ref(),
            after,
            &self.tz,
        );
        if next.is_none()
            && let Err(error) = self.store.set_status(id, Status::Done)
        {
            tracing::warn!(%error, reminder = id, "a finished reminder could not be marked done");
        }
        self.queue_fire(id, next);
    }

    fn queue_fire(&mut self, id: i64, next: Option<Fire>) {
        if let Err(error) = self.store.set_next_fire_at(id, next.map(|fire| fire.at)) {
            tracing::warn!(%error, reminder = id, "the next fire time could not be stored");
        }
        if let Some(fire) = next {
            self.queue.insert((fire.at, id));
            self.pending.insert(id, fire);
        }
    }

    fn unschedule(&mut self, id: i64) {
        if let Some(fire) = self.pending.remove(&id) {
            self.queue.remove(&(fire.at, id));
        }
    }
}

/// The start of tomorrow in `tz`. If the zone cannot say — a date at the end
/// of jiff's range — a day from now, which only means one extra rebuild.
fn next_midnight(now: Timestamp, tz: &TimeZone) -> Timestamp {
    now.to_zoned_datetime(tz.clone())
        .tomorrow()
        .and_then(|day| day.start_of_day())
        .map(|midnight| midnight.timestamp())
        .unwrap_or_else(|_| {
            now.checked_add(SignedDuration::from_hours(24))
                .unwrap_or(now)
        })
}

/// Whether the wall clock moved by something other than the time that
/// passed: `elapsed` is what the monotonic clock measured between two
/// readings of the wall clock, `before` and `after`.
pub fn clock_jumped(before: Timestamp, after: Timestamp, elapsed: Duration) -> bool {
    let elapsed = SignedDuration::try_from(elapsed).unwrap_or(SignedDuration::MAX);
    let drift = after.duration_since(before) - elapsed;
    drift.abs() > JUMP_TOLERANCE
}

/// The running scheduler thread. Dropping every [`Sender`] of its commands,
/// or the UI's receiver of its fires, ends it.
#[derive(Debug)]
pub struct Scheduler {
    commands: Sender<Command>,
    _thread: JoinHandle<()>,
}

impl Scheduler {
    /// Start the thread. It takes the store for its own: the UI opens its own
    /// connection (`ot_store`'s WAL lets both read and write).
    pub fn spawn(store: Store, fires: SyncSender<Fired>) -> std::io::Result<Self> {
        let (commands, incoming) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("scheduler".into())
            .spawn(move || run(store, &fires, &incoming))?;
        Ok(Self {
            commands,
            _thread: thread,
        })
    }

    /// Where to send edits and the platform's events.
    pub fn commands(&self) -> Sender<Command> {
        self.commands.clone()
    }
}

fn run(store: Store, fires: &SyncSender<Fired>, commands: &Receiver<Command>) {
    let mut tz = TimeZone::system();
    let mut now = Timestamp::now();
    let mut core = Core::new(store, now, tz.clone());
    tracing::info!(next = ?core.next().map(|(id, fire)| (id, fire.at)), "the scheduler started");

    loop {
        if core.tick(now, fires).is_err() {
            break;
        }

        let started = Instant::now();
        let mut received = Vec::new();
        match commands.recv_timeout(core.wait(now)) {
            Ok(command) => received.push(command),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        received.extend(commands.try_iter());

        // Every wake re-reads the wall clock rather than assuming the wait
        // lasted as long as asked.
        let before = now;
        now = Timestamp::now();
        if clock_jumped(before, now, started.elapsed()) {
            received.push(Command::Recompute(Recompute::ClockChanged));
        }
        if received.iter().any(|command| {
            matches!(
                command,
                Command::Recompute(Recompute::ClockChanged | Recompute::Resumed)
            )
        }) {
            // jiff keeps the system zone for five minutes; a clock change may
            // be a zone change, so it is asked again now.
            jiff::tz::reset_system_time_zone_cache();
        }
        let current = TimeZone::system();
        if current != tz {
            tz = current;
            core.set_zone(tz.clone(), now);
        }

        let mut rebuilt = false;
        for command in received {
            if let Command::Recompute(_) = command {
                // One rebuild answers any number of reasons for one.
                if rebuilt {
                    continue;
                }
                rebuilt = true;
            }
            core.apply(command, now);
        }
    }
    tracing::info!("the scheduler stopped");
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil;
    use ot_store::reminders::{Recurrence, RecurrenceUnit, ReminderDraft, Trigger};

    fn instant(text: &str) -> Timestamp {
        text.parse().expect("an instant")
    }

    fn at(text: &str) -> Trigger {
        Trigger::At {
            at: text.parse::<civil::DateTime>().expect("a civil time"),
        }
    }

    fn draft(trigger: Trigger, recurrence: Option<Recurrence>) -> ReminderDraft {
        ReminderDraft {
            category_id: None,
            title: "Minum air".to_owned(),
            body: None,
            authored_lang: "ms-MY".to_owned(),
            trigger,
            recurrence,
        }
    }

    fn daily() -> Option<Recurrence> {
        Some(Recurrence {
            every: 1,
            unit: RecurrenceUnit::Days,
            weekdays: Vec::new(),
        })
    }

    fn drain(fires: &Receiver<Fired>) -> Vec<(i64, Timestamp)> {
        fires
            .try_iter()
            .map(|fired| (fired.reminder_id, fired.fire.at))
            .collect()
    }

    #[test]
    fn the_queue_fires_in_time_order_and_nothing_early() {
        let store = Store::open_in_memory().expect("opens");
        let late = store
            .create_reminder(&draft(at("2026-11-19T11:00"), None))
            .expect("creates");
        let early = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);

        assert_eq!(core.next().map(|(id, _)| id), Some(early), "earliest first");

        core.tick(instant("2026-11-19T09:59:59.999Z"), &out)
            .expect("connected");
        assert!(drain(&fires).is_empty(), "nothing before its time");

        core.tick(instant("2026-11-19T11:00:00Z"), &out)
            .expect("connected");
        assert_eq!(
            drain(&fires),
            vec![
                (early, instant("2026-11-19T10:00Z")),
                (late, instant("2026-11-19T11:00Z")),
            ]
        );
        assert_eq!(core.next(), None);
    }

    #[test]
    fn a_fired_reminder_stores_its_next_time_or_is_done() {
        let store = Store::open_in_memory().expect("opens");
        let once = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let every_day = store
            .create_reminder(&draft(at("2026-11-19T10:00"), daily()))
            .expect("creates");
        let (out, _fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);

        core.tick(instant("2026-11-19T10:00:00.010Z"), &out)
            .expect("connected");

        let read = |id| core.store.reminder(id).expect("reads").expect("exists");
        assert_eq!(read(once).status, Status::Done);
        assert_eq!(read(once).next_fire_at, None);
        assert_eq!(read(every_day).status, Status::Active);
        assert_eq!(
            read(every_day).next_fire_at,
            Some(instant("2026-11-20T10:00Z"))
        );
    }

    #[test]
    fn an_edit_moves_a_queued_reminder() {
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let (out, fires) = fire_channel();
        let now = instant("2026-11-19T09:00Z");
        let mut core = Core::new(store, now, TimeZone::UTC);

        core.store
            .update_reminder(id, &draft(at("2026-11-19T10:30"), None))
            .expect("updates");
        core.apply(Command::Changed(id), now);

        core.tick(instant("2026-11-19T10:00:01Z"), &out)
            .expect("connected");
        assert!(drain(&fires).is_empty(), "not at the old time");
        core.tick(instant("2026-11-19T10:30:00Z"), &out)
            .expect("connected");
        assert_eq!(drain(&fires), vec![(id, instant("2026-11-19T10:30Z"))]);
    }

    #[test]
    fn a_removed_reminder_never_fires() {
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let (out, fires) = fire_channel();
        let now = instant("2026-11-19T09:00Z");
        let mut core = Core::new(store, now, TimeZone::UTC);

        core.apply(Command::Removed(id), now);
        core.tick(instant("2026-11-19T10:00:01Z"), &out)
            .expect("connected");
        assert!(drain(&fires).is_empty());
    }

    #[test]
    fn a_resume_rebuilds_from_the_database_rather_than_trusting_the_queue() {
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);

        // The row changes behind the queue's back, as nothing in a running
        // app should allow — the point is to prove the resume reads it.
        core.store
            .update_reminder(id, &draft(at("2026-11-19T11:00"), None))
            .expect("updates");
        core.apply(
            Command::Recompute(Recompute::Resumed),
            instant("2026-11-19T09:30Z"),
        );

        core.tick(instant("2026-11-19T10:00:01Z"), &out)
            .expect("connected");
        assert!(drain(&fires).is_empty(), "the stale 10:00 entry is gone");
        core.tick(instant("2026-11-19T11:00Z"), &out)
            .expect("connected");
        assert_eq!(drain(&fires), vec![(id, instant("2026-11-19T11:00Z"))]);
    }

    #[test]
    fn a_clock_jumped_forward_fires_what_it_skipped_once() {
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(at("2026-11-19T10:00"), daily()))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);

        // Three days later in one step.
        let later = instant("2026-11-22T12:00Z");
        core.apply(Command::Recompute(Recompute::ClockChanged), later);
        core.tick(later, &out).expect("connected");

        assert_eq!(drain(&fires), vec![(id, instant("2026-11-19T10:00Z"))]);
        assert_eq!(
            core.next().map(|(_, fire)| fire.at),
            Some(instant("2026-11-23T10:00Z")),
            "the next is the next one after now, not the backlog"
        );
    }

    #[test]
    fn a_clock_set_back_does_not_fire_a_finished_reminder_again() {
        let store = Store::open_in_memory().expect("opens");
        let once = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let every_day = store
            .create_reminder(&draft(at("2026-11-19T10:00"), daily()))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);
        core.tick(instant("2026-11-19T10:00Z"), &out)
            .expect("connected");
        assert_eq!(drain(&fires).len(), 2);

        // Back an hour: the wall clock reads 10:00 again before long.
        let back = instant("2026-11-19T09:05Z");
        core.apply(Command::Recompute(Recompute::ClockChanged), back);
        core.tick(instant("2026-11-19T10:00Z"), &out)
            .expect("connected");
        assert_eq!(
            drain(&fires),
            vec![(every_day, instant("2026-11-19T10:00Z"))],
            "the daily one follows the wall clock; the one-off is done"
        );
        let read = core.store.reminder(once).expect("reads").expect("exists");
        assert_eq!(read.status, Status::Done);
    }

    #[test]
    fn a_new_zone_moves_every_wall_clock_time() {
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let now = instant("2026-11-19T00:00Z");
        let mut core = Core::new(store, now, TimeZone::UTC);
        assert_eq!(
            core.next().map(|(_, fire)| fire.at),
            Some(instant("2026-11-19T10:00Z"))
        );

        let kl = TimeZone::get("Asia/Kuala_Lumpur").expect("bundled zone");
        core.set_zone(kl, now);
        assert_eq!(
            core.next(),
            Some((
                id,
                Fire {
                    at: instant("2026-11-19T02:00Z"),
                    occurrence: instant("2026-11-19T02:00Z"),
                    offset: None,
                }
            )),
            "10:00 in Kuala Lumpur is 02:00 UTC"
        );
    }

    #[test]
    fn midnight_rebuilds_the_queue() {
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(at("2026-11-20T10:00"), None))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T22:00Z"), TimeZone::UTC);
        assert_eq!(
            core.wait(instant("2026-11-19T23:59:50Z")),
            Duration::from_secs(10),
            "the wait ends at midnight, not at the fire"
        );

        core.store
            .update_reminder(id, &draft(at("2026-11-20T09:00"), None))
            .expect("updates");
        core.tick(instant("2026-11-20T00:00Z"), &out)
            .expect("connected");
        assert_eq!(
            core.next().map(|(_, fire)| fire.at),
            Some(instant("2026-11-20T09:00Z")),
            "rebuilt at midnight, so the changed row is read"
        );
        assert!(drain(&fires).is_empty());
    }

    #[test]
    fn a_full_channel_holds_the_fire_rather_than_dropping_it() {
        let store = Store::open_in_memory().expect("opens");
        let first = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let second = store
            .create_reminder(&draft(at("2026-11-19T10:00:05"), None))
            .expect("creates");
        let (out, fires) = mpsc::sync_channel(1);
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);

        core.tick(instant("2026-11-19T10:00:05Z"), &out)
            .expect("connected");
        assert_eq!(core.held(), 1, "no room for the second");
        assert_eq!(core.wait(instant("2026-11-19T10:00:05Z")), HELD_RETRY);

        assert_eq!(drain(&fires), vec![(first, instant("2026-11-19T10:00Z"))]);
        core.tick(instant("2026-11-19T10:00:06Z"), &out)
            .expect("connected");
        assert_eq!(
            drain(&fires),
            vec![(second, instant("2026-11-19T10:00:05Z"))]
        );
        assert_eq!(core.held(), 0);
    }

    #[test]
    fn a_gone_ui_stops_the_scheduler() {
        let store = Store::open_in_memory().expect("opens");
        store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);
        drop(fires);
        assert_eq!(
            core.tick(instant("2026-11-19T10:00Z"), &out),
            Err(Disconnected)
        );
    }

    #[test]
    fn the_wait_is_bounded_however_far_the_next_fire_is() {
        let store = Store::open_in_memory().expect("opens");
        store
            .create_reminder(&draft(at("2027-01-01T00:00"), None))
            .expect("creates");
        let now = instant("2026-11-19T09:00Z");
        let core = Core::new(store, now, TimeZone::UTC);
        assert_eq!(core.wait(now), MAX_WAIT);
        assert_eq!(
            core.wait(instant("2026-11-19T23:59:59.5Z")),
            Duration::from_millis(500)
        );
    }

    #[test]
    fn a_clock_jump_is_drift_beyond_the_tolerance_either_way() {
        let before = instant("2026-11-19T09:00Z");
        let ten = Duration::from_secs(10);
        assert!(!clock_jumped(before, instant("2026-11-19T09:00:10Z"), ten));
        assert!(!clock_jumped(
            before,
            instant("2026-11-19T09:00:11.5Z"),
            ten
        ));
        assert!(clock_jumped(before, instant("2026-11-19T10:00:10Z"), ten));
        assert!(clock_jumped(before, instant("2026-11-19T08:00:10Z"), ten));
    }

    #[test]
    fn the_thread_fires_within_a_second_of_target() {
        // Real clock, real thread: a reminder 1.5 s out, timed against its
        // target. The wall clock is the system zone's, as in the app.
        let tz = TimeZone::system();
        let target = Timestamp::now()
            .checked_add(SignedDuration::from_millis(1500))
            .expect("in range");
        let civil = target.to_zoned_datetime(tz).datetime();
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(Trigger::At { at: civil }, None))
            .expect("creates");

        let (out, fires) = fire_channel();
        let scheduler = Scheduler::spawn(store, out).expect("starts");
        let fired = fires.recv_timeout(Duration::from_secs(10)).expect("fires");
        let received = Timestamp::now();

        assert_eq!(fired.reminder_id, id);
        let late = received.duration_since(fired.fire.at);
        assert!(
            late >= SignedDuration::ZERO && late < SignedDuration::from_secs(1),
            "fired {late:?} after its target"
        );
        drop(scheduler);
    }
}
