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
use ot_store::reminders::{NotificationAction, Reminder, Status, Template};

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

/// How late a fire may be and still be a toast. Later than this, the machine
/// was off or asleep or the clock jumped, and the fire is a missed one
/// (`docs/specs/reminders.md` §6).
pub const MISSED_AFTER: SignedDuration = SignedDuration::from_secs(60);

/// The catch-up window for a reminder until the settings say otherwise.
pub const DEFAULT_MISSED_WINDOW_HOURS: u32 = 24;

/// The catch-up window for a missed prayer until the settings say otherwise:
/// an Azan an hour late is still worth knowing about, a day late is not
/// (`docs/11-prayer-and-azan.md` §9, `docs/specs/reminders.md` §6).
pub const DEFAULT_AZAN_WINDOW_MINUTES: u32 = 60;

/// Where a fire sits when two are due at the same instant: Azan, then an
/// Alarm, then everything else (`docs/specs/reminders.md` §8).
const RANK_PRAYER: u8 = 0;
const RANK_ALARM: u8 = 1;
const RANK_OTHER: u8 = 2;

/// The built-in type whose reminders rank as Alarms.
const ALARM_TYPE: &str = "reminder-alarm";

/// How long the PC may be off or asleep, or the scheduler not running, before
/// the working day the fatigue reminders count from ends and the next one
/// starts (`docs/specs/reminders.md` §3).
pub const WORKDAY_GAP: SignedDuration = SignedDuration::from_hours(1);

/// How often the working day's "still here" mark is written to the database.
const ALIVE_EVERY: SignedDuration = SignedDuration::from_secs(60);

/// The `settings` keys the working day survives a restart in.
const WORKDAY_START: &str = "fatigue.workday_start";
const WORKDAY_ALIVE: &str = "fatigue.last_alive";

/// The most skipped occurrences of one reminder walked through to find its
/// latest, so a five-minute reminder off for a year cannot stall the thread.
const MAX_SKIPPED: u32 = 10_000;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// A reminder was created or edited: read it again and requeue it.
    Changed(i64),
    /// A reminder was deleted.
    Removed(i64),
    /// Throw the queue away and rebuild it from the database.
    Recompute(Recompute),
    /// Write a `notification_log` row: what became of a fire. The UI sends
    /// these, since only this thread holds the store.
    Record {
        reminder_id: i64,
        fired_at: Timestamp,
        action: NotificationAction,
    },
    /// Snooze a fire for `minutes` from now. `snoozes` is how many times the
    /// fire being snoozed has been snoozed already; a fire at `limit` is
    /// refused (`docs/specs/reminders.md` §5).
    Snooze {
        reminder_id: i64,
        fired_at: Timestamp,
        minutes: u32,
        snoozes: u32,
        limit: u32,
    },
    /// The catch-up window changed in Settings ▸ Reminders.
    SetMissedWindow { hours: u32 },
    /// The day's prayer times, by prayer key (`ot_store::reminders::PRAYERS`):
    /// every time to fire, today's and tomorrow's, replacing what was held.
    /// The scheduler does not compute them (`docs/11-prayer-and-azan.md` §9);
    /// the app resolves them and says when they change.
    PrayerTimes(Vec<(String, Timestamp)>),
    /// Which prayers have a reminder: the keys that are on. The rows are made
    /// and removed to match, and the queue is rebuilt.
    EnablePrayers(Vec<String>),
    /// The catch-up window for a missed prayer, in minutes.
    SetAzanWindow { minutes: u32 },
    /// Which prayers have a reminder `minutes` ahead of them. The rows are made,
    /// moved and removed to match, and the queue is rebuilt.
    EnablePrePrayers { keys: Vec<String>, minutes: u32 },
}

/// One fire, sent to the UI with what it needs to build the toast, so the UI
/// never reads the database on its own thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fired {
    pub reminder_id: i64,
    pub fire: Fire,
    /// The reminder as it stood when it fired.
    pub reminder: Reminder,
    /// Its template, if it names one.
    pub template: Option<Template>,
    /// How many times this fire has been snoozed: 0 for a reminder's own
    /// time, 1 for the first snooze's return, and so on.
    pub snoozes: u32,
    /// Its time passed while the PC was off or asleep, or the clock jumped:
    /// listed with the missed reminders rather than shown as a toast.
    pub missed: bool,
}

/// The UI's receiving end has gone, so there is no one left to fire to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disconnected;

/// The bounded channel fires travel on, scheduler to UI.
pub fn fire_channel() -> (SyncSender<Fired>, Receiver<Fired>) {
    mpsc::sync_channel(FIRE_CAPACITY)
}

/// Which of a reminder's fires a queue entry is: its own schedule, or a
/// snooze of an earlier fire. A reminder may have one of each waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Regular,
    Snooze,
}

/// The queue and the rules about it, with no clock and no thread of its own.
#[derive(Debug)]
pub struct Core {
    store: Store,
    tz: TimeZone,
    /// Ordered by fire time, then by rank (Azan, Alarm, the rest), then id.
    queue: BTreeSet<(Timestamp, u8, i64, Kind)>,
    /// Each queued reminder's rank, read when it is scheduled.
    ranks: BTreeMap<i64, u8>,
    /// Every prayer's times to fire at, by key; given by the app.
    prayer_times: BTreeMap<String, Vec<Timestamp>>,
    /// How old a missed prayer may be and still be listed.
    azan_window: SignedDuration,
    /// Each queued fire, by reminder and kind.
    pending: BTreeMap<(i64, Kind), Fire>,
    /// Each waiting snooze's place in its chain.
    chain: BTreeMap<i64, u32>,
    /// Fires the UI's channel had no room for, oldest first.
    held: VecDeque<Fired>,
    /// Everything up to this instant has been fired. A rebuild starts from
    /// here, so a clock jumped forward fires what it jumped over, once.
    horizon: Timestamp,
    /// The next local midnight, when the queue is rebuilt.
    midnight: Timestamp,
    /// How old a missed fire may be and still be listed; older ones are
    /// dropped with a log line and a `missed` row.
    missed_window: SignedDuration,
    /// When the working day the fatigue reminders count from began, and
    /// when the scheduler last saw the PC in use; a gap of [`WORKDAY_GAP`]
    /// between that and now starts a new day.
    workday_start: Timestamp,
    last_alive: Timestamp,
    /// When `last_alive` was last written to the database.
    alive_written: Timestamp,
}

impl Core {
    /// Build the queue from every active or snoozed reminder, as of `now`.
    pub fn new(store: Store, now: Timestamp, tz: TimeZone) -> Self {
        let midnight = next_midnight(now, &tz);
        let mut core = Self {
            store,
            tz,
            queue: BTreeSet::new(),
            ranks: BTreeMap::new(),
            prayer_times: BTreeMap::new(),
            azan_window: SignedDuration::from_mins(i64::from(DEFAULT_AZAN_WINDOW_MINUTES)),
            pending: BTreeMap::new(),
            chain: BTreeMap::new(),
            held: VecDeque::new(),
            horizon: now,
            midnight,
            missed_window: SignedDuration::from_hours(i64::from(DEFAULT_MISSED_WINDOW_HOURS)),
            workday_start: now,
            last_alive: now,
            alive_written: now,
        };
        core.resume_workday(now);
        core.rebuild(now);
        core
    }

    /// When the current working day began.
    pub fn workday_start(&self) -> Timestamp {
        self.workday_start
    }

    /// Carry on the working day a restart interrupted, if the app was gone
    /// for less than [`WORKDAY_GAP`]; otherwise this start begins a new one.
    fn resume_workday(&mut self, now: Timestamp) {
        let read = |key| {
            self.store
                .setting(key)
                .ok()
                .flatten()
                .and_then(|text| text.parse::<Timestamp>().ok())
        };
        if let (Some(start), Some(alive)) = (read(WORKDAY_START), read(WORKDAY_ALIVE))
            && start <= now
            && now.duration_since(alive) <= WORKDAY_GAP
        {
            self.workday_start = start;
        }
        self.write_workday(now);
    }

    fn write_workday(&mut self, now: Timestamp) {
        self.last_alive = now;
        self.alive_written = now;
        for (key, value) in [(WORKDAY_START, self.workday_start), (WORKDAY_ALIVE, now)] {
            if let Err(error) = self
                .store
                .set_setting(key, &ot_store::reminders::stamp(value))
            {
                tracing::warn!(%error, "the working day could not be stored");
            }
        }
    }

    /// Note that the PC is in use at `now`. After a gap of [`WORKDAY_GAP`] —
    /// asleep, off, or the clock jumped — the working day starts again here,
    /// and the fatigue reminders count from it. Midnight does not end a day.
    fn observe(&mut self, now: Timestamp) {
        let gap = now.duration_since(self.last_alive);
        if gap > WORKDAY_GAP || gap < -WORKDAY_GAP {
            tracing::info!(gap = %gap, "a new working day began");
            self.workday_start = now;
            self.write_workday(now);
            self.reschedule_fatigue(now);
            return;
        }
        self.last_alive = now;
        if now.duration_since(self.alive_written) >= ALIVE_EVERY {
            self.write_workday(now);
        }
    }

    /// Requeue every fatigue reminder from the working day's start.
    fn reschedule_fatigue(&mut self, now: Timestamp) {
        match self.store.reminders() {
            Ok(reminders) => {
                for reminder in reminders.iter().filter(|r| r.draft.trigger.is_fatigue()) {
                    self.schedule(reminder, now);
                }
            }
            Err(error) => tracing::warn!(%error, "the reminders could not be read"),
        }
    }

    /// Set the catch-up window, in hours (Settings ▸ Reminders).
    pub fn set_missed_window(&mut self, hours: u32) {
        self.missed_window = SignedDuration::from_hours(i64::from(hours));
    }

    /// The next fire waiting, if any.
    pub fn next(&self) -> Option<(i64, Fire)> {
        let &(_, _, id, kind) = self.queue.first()?;
        self.pending.get(&(id, kind)).map(|fire| (id, *fire))
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
            Command::Record {
                reminder_id,
                fired_at,
                action,
            } => self.record(reminder_id, fired_at, action),
            Command::Snooze {
                reminder_id,
                fired_at,
                minutes,
                snoozes,
                limit,
            } => self.snooze(reminder_id, fired_at, minutes, snoozes, limit, now),
            Command::SetMissedWindow { hours } => self.set_missed_window(hours),
            Command::SetAzanWindow { minutes } => {
                self.azan_window = SignedDuration::from_mins(i64::from(minutes));
            }
            Command::PrayerTimes(times) => self.set_prayer_times(times, now),
            Command::EnablePrePrayers { keys, minutes } => {
                let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
                match self.store.sync_pre_prayer_reminders(&keys, minutes) {
                    Ok(true) => {
                        tracing::info!(?keys, minutes, "the pre-prayer reminders were changed");
                        self.rebuild(now);
                    }
                    Ok(false) => {}
                    Err(error) => {
                        tracing::warn!(%error, "the pre-prayer reminders could not be changed");
                    }
                }
            }
            Command::EnablePrayers(keys) => {
                let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
                match self.store.sync_prayer_reminders(&keys) {
                    Ok(true) => {
                        tracing::info!(?keys, "the prayer reminders were changed");
                        self.rebuild(now);
                    }
                    Ok(false) => {}
                    Err(error) => {
                        tracing::warn!(%error, "the prayer reminders could not be changed");
                    }
                }
            }
        }
    }

    /// Take the day's prayer times and queue every prayer's next one from
    /// them. Replaces what was held: a zone, a place or a fetch that moved a
    /// time moves the fire.
    fn set_prayer_times(&mut self, times: Vec<(String, Timestamp)>, now: Timestamp) {
        let mut held: BTreeMap<String, Vec<Timestamp>> = BTreeMap::new();
        for (prayer, at) in times {
            held.entry(prayer).or_default().push(at);
        }
        for list in held.values_mut() {
            list.sort();
            list.dedup();
        }
        tracing::info!(
            prayers = held.len(),
            times = held.values().map(Vec::len).sum::<usize>(),
            "prayer times received"
        );
        self.prayer_times = held;
        let after = self.horizon.min(now);
        match self.store.reminders() {
            Ok(reminders) => {
                for reminder in reminders.iter().filter(|r| r.draft.trigger.is_prayer()) {
                    self.schedule(reminder, after);
                }
            }
            Err(error) => tracing::warn!(%error, "the prayer reminders could not be read"),
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
        self.observe(now);
        if now >= self.midnight {
            self.apply(Command::Recompute(Recompute::Midnight), now);
        }

        while let Some(&(at, _, id, kind)) = self.queue.first() {
            if at > now {
                break;
            }
            self.queue.pop_first();
            let Some(fire) = self.pending.remove(&(id, kind)) else {
                continue;
            };
            let snoozes = match kind {
                Kind::Regular => 0,
                Kind::Snooze => self.chain.remove(&id).unwrap_or(1),
            };
            // Read again at the moment of firing, so the toast says what the
            // reminder says now. A row gone, done or cancelled does not fire.
            let reminder = match self.store.reminder(id) {
                Ok(Some(reminder))
                    if matches!(reminder.status, Status::Active | Status::Snoozed) =>
                {
                    reminder
                }
                Ok(_) => continue,
                Err(error) => {
                    tracing::warn!(%error, reminder = id, "a due reminder could not be read");
                    continue;
                }
            };
            // A fire this late was missed. Only the latest of a recurring
            // reminder's skipped occurrences is listed, and only if it is
            // inside the catch-up window; the rest are dropped with a log
            // line and one `missed` row (docs/specs/reminders.md §6).
            let late = now.duration_since(fire.at) > MISSED_AFTER;
            // A fatigue fire that came due while the PC was away is not
            // worth catching up on: it is skipped, and the next one queued.
            if late && kind == Kind::Regular && reminder.draft.trigger.is_fatigue() {
                tracing::debug!(reminder = id, "a late fatigue reminder was skipped");
                self.requeue_after_fire(&reminder, now);
                continue;
            }
            // A prayer is its own occurrence: there is no run of skipped ones to
            // walk, and its window is the Azan's.
            let prayer = reminder.draft.trigger.is_prayer();
            // A reminder ahead of a prayer that came due late is moot — the
            // prayer is here or past — so it is skipped, not listed.
            if late && kind == Kind::Regular && reminder.draft.trigger.is_pre_prayer() {
                tracing::debug!(reminder = id, "a late pre-prayer reminder was skipped");
                self.requeue_after_fire(&reminder, now);
                continue;
            }
            let (fire, dropped) = if late && kind == Kind::Regular && !prayer {
                self.latest_skipped(&reminder, fire, now)
            } else {
                (fire, None)
            };
            let window = if prayer {
                self.azan_window
            } else {
                self.missed_window
            };
            let listed = !late || now.duration_since(fire.at) <= window;
            let dropped = if listed {
                dropped
            } else {
                Some((fire.at, dropped.map_or(1, |(_, n)| n + 1)))
            };
            if let Some((newest, count)) = dropped {
                tracing::info!(
                    reminder = id,
                    count,
                    newest = %newest,
                    "missed fires older than the catch-up window were dropped"
                );
                self.record(id, newest, NotificationAction::Missed);
            }
            let template = reminder.draft.category_id.and_then(|category| {
                self.store
                    .template(category)
                    .inspect_err(|error| {
                        tracing::warn!(%error, reminder = id, "a reminder's template could not be read");
                    })
                    .ok()
                    .flatten()
            });
            match kind {
                // The next fire counts from now, not from the one just taken,
                // so a clock jumped over three days fires a daily reminder
                // once, not three times (catching up properly is
                // docs/specs/reminders.md §6, FEAT-0045).
                Kind::Regular => self.requeue_after_fire(&reminder, fire.at.max(now)),
                // The snooze is spent: the reminder is active again if its
                // own schedule has more, and done if not.
                Kind::Snooze => {
                    let status = if self.pending.contains_key(&(id, Kind::Regular)) {
                        Status::Active
                    } else {
                        Status::Done
                    };
                    self.set_status(id, status);
                    self.store_next(id);
                }
            }
            if listed {
                self.held.push_back(Fired {
                    reminder_id: id,
                    fire,
                    reminder,
                    template,
                    snoozes,
                    missed: late,
                });
            }
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
        if let Some(&(at, _, _, _)) = self.queue.first() {
            until = until.min(at);
        }
        Duration::try_from(until.duration_since(now))
            .unwrap_or(Duration::ZERO)
            .min(MAX_WAIT)
    }

    /// Offer held fires to the UI, oldest first, until its channel is full.
    fn flush(&mut self, now: Timestamp, out: &SyncSender<Fired>) -> Result<(), Disconnected> {
        while let Some(fired) = self.held.pop_front() {
            let (id, target) = (fired.reminder_id, fired.fire.at);
            match out.try_send(fired) {
                Ok(()) => {
                    // The line the timing check reads: the target and how far
                    // past it the fire left (docs/specs/reminders.md §7).
                    let late_ms = now.duration_since(target).as_millis();
                    tracing::info!(
                        reminder = id,
                        target = %target,
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
        self.chain.clear();
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

    /// Walk a recurring reminder's occurrences from `first` to the latest at
    /// or before `now`. Returns that one, and the newest skipped before it
    /// with how many were skipped, if any.
    fn latest_skipped(
        &self,
        reminder: &Reminder,
        first: Fire,
        now: Timestamp,
    ) -> (Fire, Option<(Timestamp, u32)>) {
        let mut latest = first;
        let mut skipped: Option<(Timestamp, u32)> = None;
        while let Some(next) = next_fire(
            &reminder.draft.trigger,
            reminder.draft.recurrence.as_ref(),
            latest.at,
            &self.tz,
        ) {
            if next.at > now || skipped.is_some_and(|(_, n)| n >= MAX_SKIPPED) {
                break;
            }
            skipped = Some((latest.at, skipped.map_or(1, |(_, n)| n + 1)));
            latest = next;
        }
        (latest, skipped)
    }

    /// Queue a reminder's own next fire after `after`, if it is active or
    /// snoozed and has one, and its waiting snooze if it is snoozed — read
    /// from the snooze log, which is how a snooze survives a restart.
    ///
    /// A stored next fire time earlier than `after` is a fire nobody saw —
    /// the app was not running, or the machine slept — so the queue starts
    /// from it, and `tick` finds it missed rather than skipping it.
    fn schedule(&mut self, reminder: &Reminder, after: Timestamp) {
        let id = reminder.id;
        self.unschedule(id);
        if !matches!(reminder.status, Status::Active | Status::Snoozed) {
            return;
        }
        let rank = self.rank_of(reminder);
        self.ranks.insert(id, rank);
        let after = match reminder.next_fire_at {
            Some(stored) if stored <= after => stored
                .checked_sub(SignedDuration::from_nanos(1))
                .unwrap_or(stored),
            _ => after,
        };
        let next = self.next_of(reminder, after);
        if let Some(fire) = next {
            self.insert(id, Kind::Regular, fire);
        }
        if reminder.status == Status::Snoozed {
            match self.store.latest_snooze(id) {
                Ok(Some((until, count))) => self.insert_snooze(id, until, count),
                Ok(None) => {}
                Err(error) => tracing::warn!(%error, reminder = id, "a snooze could not be read"),
            }
        }
        self.store_next(id);
    }

    /// After a regular fire: queue the next one, or mark the reminder done if
    /// that was its last and no snooze is waiting, so a clock set back cannot
    /// fire it a second time.
    fn requeue_after_fire(&mut self, reminder: &Reminder, after: Timestamp) {
        let id = reminder.id;
        let next = self.next_of(reminder, after);
        match next {
            // A fatigue schedule is never done: the work-hour limit has no
            // more fires today, and comes again with the next working day.
            // Nor is a prayer: its next time comes with the next day's times.
            None if reminder.draft.trigger.is_fatigue() || reminder.draft.trigger.is_prayer() => {}
            Some(fire) => self.insert(id, Kind::Regular, fire),
            None if !self.pending.contains_key(&(id, Kind::Snooze)) => {
                self.set_status(id, Status::Done);
            }
            None => {}
        }
        self.store_next(id);
    }

    /// A reminder's own next fire after `after`: from its date and repeat,
    /// or, for a fatigue schedule, from the working day.
    fn next_of(&self, reminder: &Reminder, after: Timestamp) -> Option<Fire> {
        if reminder.draft.trigger.is_fatigue() {
            return crate::fatigue::next_fire(&reminder.draft.trigger, self.workday_start, after);
        }
        if let ot_store::reminders::Trigger::Prayer { prayer } = &reminder.draft.trigger {
            return self.prayer_next(reminder, prayer, after);
        }
        if let ot_store::reminders::Trigger::PrayerBefore { prayer, minutes } =
            &reminder.draft.trigger
        {
            return self.pre_prayer_next(reminder, prayer, *minutes, after);
        }
        next_fire(
            &reminder.draft.trigger,
            reminder.draft.recurrence.as_ref(),
            after,
            &self.tz,
        )
    }

    /// Put a fire off. Refused at the limit, and for a reminder that is gone.
    fn snooze(
        &mut self,
        id: i64,
        fired_at: Timestamp,
        minutes: u32,
        snoozes: u32,
        limit: u32,
        now: Timestamp,
    ) {
        if snoozes >= limit {
            tracing::warn!(
                reminder = id,
                snoozes,
                limit,
                "a snooze past the limit was refused"
            );
            return;
        }
        let Ok(until) = now.checked_add(SignedDuration::from_mins(i64::from(minutes))) else {
            return;
        };
        match self.store.reminder(id) {
            Ok(Some(_)) => {}
            Ok(None) => return,
            Err(error) => {
                tracing::warn!(%error, reminder = id, "a reminder to snooze could not be read");
                return;
            }
        }
        let count = snoozes + 1;
        if let Err(error) = self.store.record_snooze(id, until, count) {
            tracing::warn!(%error, reminder = id, "a snooze could not be recorded");
            return;
        }
        self.set_status(id, Status::Snoozed);
        self.record(id, fired_at, NotificationAction::Snoozed);
        self.insert_snooze(id, until, count);
        self.store_next(id);
        tracing::info!(reminder = id, until = %until, count, "a reminder was snoozed");
    }

    fn insert(&mut self, id: i64, kind: Kind, fire: Fire) {
        let rank = self.ranks.get(&id).copied().unwrap_or(RANK_OTHER);
        if let Some(old) = self.pending.insert((id, kind), fire) {
            self.queue.remove(&(old.at, rank, id, kind));
        }
        self.queue.insert((fire.at, rank, id, kind));
    }

    /// Azan before Alarm before everything else, when two are due together.
    fn rank_of(&self, reminder: &Reminder) -> u8 {
        if reminder.draft.trigger.is_prayer() {
            return RANK_PRAYER;
        }
        let alarm = reminder
            .draft
            .category_id
            .and_then(|category| self.store.template(category).ok().flatten())
            .is_some_and(|template| template.type_key.as_deref() == Some(ALARM_TYPE));
        if alarm { RANK_ALARM } else { RANK_OTHER }
    }

    /// The next fire of a reminder `minutes` before a prayer: from the prayer's
    /// own times, each moved earlier, so it moves when the prayer does. The
    /// fire's `occurrence` is the prayer and its `offset` the lead, which is how
    /// the toast says "in 10 minutes".
    fn pre_prayer_next(
        &self,
        reminder: &Reminder,
        prayer: &str,
        minutes: u32,
        after: Timestamp,
    ) -> Option<Fire> {
        let lead = SignedDuration::from_mins(i64::from(minutes));
        let offset = Some(ot_store::reminders::Offset {
            amount: minutes,
            unit: ot_store::reminders::OffsetUnit::Minutes,
        });
        let given = self
            .prayer_times
            .get(prayer)
            .and_then(|times| times.iter().find(|at| **at - lead > after))
            .map(|prayer_at| (*prayer_at - lead, *prayer_at));
        let (at, occurrence) = given.or_else(|| {
            // Before any times are given, the stored time stands.
            reminder
                .next_fire_at
                .filter(|stored| *stored > after)
                .map(|stored| (stored, stored + lead))
        })?;
        Some(Fire {
            at,
            occurrence,
            offset,
        })
    }

    /// A prayer's next time after `after`: from the times the app gave, which
    /// are the truth, or — before it has given any, as at start — from the
    /// time stored with the reminder, so a prayer that came due while the app
    /// was off is still found, and found missed.
    fn prayer_next(&self, reminder: &Reminder, prayer: &str, after: Timestamp) -> Option<Fire> {
        let given = self
            .prayer_times
            .get(prayer)
            .and_then(|times| times.iter().find(|at| **at > after))
            .copied();
        let at = given.or_else(|| reminder.next_fire_at.filter(|stored| *stored > after))?;
        Some(Fire {
            at,
            occurrence: at,
            offset: None,
        })
    }

    fn insert_snooze(&mut self, id: i64, until: Timestamp, count: u32) {
        let fire = Fire {
            at: until,
            occurrence: until,
            offset: None,
        };
        self.insert(id, Kind::Snooze, fire);
        self.chain.insert(id, count);
    }

    /// Store the reminder's earliest waiting fire, of either kind, as its
    /// `next_fire_at`.
    fn store_next(&mut self, id: i64) {
        let next = [Kind::Regular, Kind::Snooze]
            .iter()
            .filter_map(|kind| self.pending.get(&(id, *kind)).map(|fire| fire.at))
            .min();
        if let Err(error) = self.store.set_next_fire_at(id, next) {
            tracing::warn!(%error, reminder = id, "the next fire time could not be stored");
        }
    }

    fn set_status(&mut self, id: i64, status: Status) {
        if let Err(error) = self.store.set_status(id, status) {
            tracing::warn!(%error, reminder = id, ?status, "a reminder's status could not be stored");
        }
    }

    fn record(&mut self, id: i64, fired_at: Timestamp, action: NotificationAction) {
        if let Err(error) = self
            .store
            .log_notification(Some(id), fired_at, action, None)
        {
            tracing::warn!(%error, reminder = id, "the notification history could not be written");
        }
    }

    fn unschedule(&mut self, id: i64) {
        let rank = self.ranks.get(&id).copied().unwrap_or(RANK_OTHER);
        for kind in [Kind::Regular, Kind::Snooze] {
            if let Some(fire) = self.pending.remove(&(id, kind)) {
                self.queue.remove(&(fire.at, rank, id, kind));
            }
        }
        self.chain.remove(&id);
    }

    /// The store back, as after a restart, for tests.
    #[cfg(test)]
    fn into_store(self) -> Store {
        self.store
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
    pub fn spawn(
        store: Store,
        fires: SyncSender<Fired>,
        missed_window_hours: u32,
    ) -> std::io::Result<Self> {
        let (commands, incoming) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("scheduler".into())
            .spawn(move || run(store, &fires, &incoming, missed_window_hours))?;
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

fn run(
    store: Store,
    fires: &SyncSender<Fired>,
    commands: &Receiver<Command>,
    missed_window_hours: u32,
) {
    let mut tz = TimeZone::system();
    let mut now = Timestamp::now();
    let mut core = Core::new(store, now, tz.clone());
    core.set_missed_window(missed_window_hours);
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
    fn a_fire_carries_the_reminder_and_its_template() {
        let store = Store::open_in_memory().expect("opens");
        let alarm = store
            .template_for_type("reminder-alarm")
            .expect("reads")
            .expect("seeded");
        let mut with_template = draft(at("2026-11-19T10:00"), None);
        with_template.category_id = Some(alarm.id);
        let id = store.create_reminder(&with_template).expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);

        core.tick(instant("2026-11-19T10:00Z"), &out)
            .expect("connected");
        let fired = fires.try_recv().expect("fired");
        assert_eq!(fired.reminder_id, id);
        assert_eq!(fired.reminder.draft.title, "Minum air", "as typed");
        assert_eq!(fired.template, Some(alarm));
    }

    #[test]
    fn a_record_command_writes_the_history_row() {
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let now = instant("2026-11-19T10:00Z");
        let mut core = Core::new(store, now, TimeZone::UTC);
        for action in [NotificationAction::Shown, NotificationAction::Opened] {
            core.apply(
                Command::Record {
                    reminder_id: id,
                    fired_at: now,
                    action,
                },
                now,
            );
        }
        assert_eq!(
            core.store.history(id).expect("reads"),
            vec![
                (now, NotificationAction::Shown),
                (now, NotificationAction::Opened)
            ]
        );
    }

    fn snooze(id: i64, fired_at: &str, minutes: u32, snoozes: u32, limit: u32) -> Command {
        Command::Snooze {
            reminder_id: id,
            fired_at: instant(fired_at),
            minutes,
            snoozes,
            limit,
        }
    }

    #[test]
    fn a_snoozed_fire_returns_after_its_minutes_with_its_count() {
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);
        core.tick(instant("2026-11-19T10:00Z"), &out)
            .expect("connected");
        assert_eq!(fires.try_recv().expect("fired").snoozes, 0);

        core.apply(
            snooze(id, "2026-11-19T10:00Z", 5, 0, 10),
            instant("2026-11-19T10:00:10Z"),
        );
        let read = core.store.reminder(id).expect("reads").expect("exists");
        assert_eq!(read.status, Status::Snoozed, "not done: it is coming back");
        assert_eq!(read.next_fire_at, Some(instant("2026-11-19T10:05:10Z")));
        assert_eq!(
            core.store.latest_snooze(id).expect("reads"),
            Some((instant("2026-11-19T10:05:10Z"), 1))
        );

        core.tick(instant("2026-11-19T10:05:09Z"), &out)
            .expect("connected");
        assert!(fires.try_recv().is_err(), "not early");
        core.tick(instant("2026-11-19T10:05:10Z"), &out)
            .expect("connected");
        let back = fires.try_recv().expect("returned");
        assert_eq!(back.snoozes, 1);
        assert_eq!(back.fire.at, instant("2026-11-19T10:05:10Z"));
        let read = core.store.reminder(id).expect("reads").expect("exists");
        assert_eq!(read.status, Status::Done, "a one-off with nothing left");
        assert!(
            core.store
                .history(id)
                .expect("reads")
                .contains(&(instant("2026-11-19T10:00Z"), NotificationAction::Snoozed))
        );
    }

    #[test]
    fn the_snooze_after_the_limit_is_refused_and_not_recorded() {
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut now = instant("2026-11-19T10:00Z");
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);
        core.tick(now, &out).expect("connected");
        let mut fired = fires.try_recv().expect("fired");

        // Three allowed, then the fourth refused: the limit counts each fire's
        // own chain of snoozes.
        for expected in 1..=3 {
            core.apply(
                Command::Snooze {
                    reminder_id: id,
                    fired_at: fired.fire.at,
                    minutes: 1,
                    snoozes: fired.snoozes,
                    limit: 3,
                },
                now,
            );
            now = now
                .checked_add(SignedDuration::from_mins(1))
                .expect("in range");
            core.tick(now, &out).expect("connected");
            fired = fires.try_recv().expect("returned");
            assert_eq!(fired.snoozes, expected);
        }
        core.apply(
            Command::Snooze {
                reminder_id: id,
                fired_at: fired.fire.at,
                minutes: 1,
                snoozes: fired.snoozes,
                limit: 3,
            },
            now,
        );
        assert_eq!(
            core.store.latest_snooze(id).expect("reads").map(|(_, n)| n),
            Some(3),
            "the fourth was not recorded"
        );
        assert_eq!(core.next(), None, "and nothing is coming back");
        let read = core.store.reminder(id).expect("reads").expect("exists");
        assert_eq!(read.status, Status::Done);
    }

    #[test]
    fn a_waiting_snooze_and_its_count_survive_a_restart() {
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);
        core.tick(instant("2026-11-19T10:00Z"), &out)
            .expect("connected");
        let _ = fires.try_recv();
        core.apply(
            snooze(id, "2026-11-19T10:00Z", 15, 4, 10),
            instant("2026-11-19T10:00Z"),
        );

        // The app quits and starts again before the snooze ends.
        let store = core.into_store();
        let mut core = Core::new(store, instant("2026-11-19T10:10Z"), TimeZone::UTC);
        assert_eq!(
            core.next().map(|(_, fire)| fire.at),
            Some(instant("2026-11-19T10:15Z"))
        );
        core.tick(instant("2026-11-19T10:15Z"), &out)
            .expect("connected");
        assert_eq!(fires.try_recv().expect("returned").snoozes, 5);
    }

    #[test]
    fn a_recurring_reminder_keeps_its_schedule_and_the_next_occurrence_starts_at_zero() {
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(at("2026-11-19T10:00"), daily()))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);
        core.tick(instant("2026-11-19T10:00Z"), &out)
            .expect("connected");
        let _ = fires.try_recv();
        core.apply(
            snooze(id, "2026-11-19T10:00Z", 60, 0, 10),
            instant("2026-11-19T10:00Z"),
        );
        let read = core.store.reminder(id).expect("reads").expect("exists");
        assert_eq!(
            read.next_fire_at,
            Some(instant("2026-11-19T11:00Z")),
            "the snooze is the earlier of its two"
        );

        core.tick(instant("2026-11-19T11:00Z"), &out)
            .expect("connected");
        assert_eq!(fires.try_recv().expect("snooze").snoozes, 1);
        let read = core.store.reminder(id).expect("reads").expect("exists");
        assert_eq!(read.status, Status::Active, "its own schedule goes on");
        assert_eq!(read.next_fire_at, Some(instant("2026-11-20T10:00Z")));

        core.tick(instant("2026-11-20T10:00Z"), &out)
            .expect("connected");
        assert_eq!(fires.try_recv().expect("tomorrow").snoozes, 0);
    }

    #[test]
    fn a_reminder_due_while_the_app_was_off_is_listed_at_the_next_start() {
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);
        // The app quits before 10:00 and starts again at 12:00.
        let store = core.into_store();
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T12:00Z"), TimeZone::UTC);
        core.tick(instant("2026-11-19T12:00Z"), &out)
            .expect("connected");
        let fired = fires.try_recv().expect("listed, not skipped");
        assert_eq!(fired.reminder_id, id);
        assert_eq!(fired.fire.at, instant("2026-11-19T10:00Z"));
        assert!(fired.missed, "a list entry, not a toast");
    }

    #[test]
    fn a_minute_late_is_a_toast_and_later_is_missed() {
        for (now, missed) in [
            ("2026-11-19T10:01:00Z", false),
            ("2026-11-19T10:01:00.001Z", true),
        ] {
            let store = Store::open_in_memory().expect("opens");
            store
                .create_reminder(&draft(at("2026-11-19T10:00"), None))
                .expect("creates");
            let (out, fires) = fire_channel();
            let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);
            core.tick(instant(now), &out).expect("connected");
            assert_eq!(fires.try_recv().expect("delivered").missed, missed, "{now}");
        }
    }

    #[test]
    fn the_catch_up_window_lists_at_its_edge_and_drops_past_it() {
        for (now, listed) in [
            ("2026-11-20T10:00:00Z", true),
            ("2026-11-20T10:00:00.001Z", false),
        ] {
            let store = Store::open_in_memory().expect("opens");
            let id = store
                .create_reminder(&draft(at("2026-11-19T10:00"), None))
                .expect("creates");
            let core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);
            let store = core.into_store();
            let (out, fires) = fire_channel();
            let mut core = Core::new(store, instant(now), TimeZone::UTC);
            core.tick(instant(now), &out).expect("connected");
            let history = core.store.history(id).expect("reads");
            if listed {
                assert!(fires.try_recv().expect("listed").missed, "{now}");
                assert!(history.is_empty(), "the UI writes the listed row");
            } else {
                assert!(fires.try_recv().is_err(), "dropped at {now}");
                assert_eq!(
                    history,
                    vec![(instant("2026-11-19T10:00Z"), NotificationAction::Missed)]
                );
            }
        }
    }

    #[test]
    fn a_shorter_window_drops_what_the_default_would_list() {
        let store = Store::open_in_memory().expect("opens");
        store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);
        core.apply(
            Command::SetMissedWindow { hours: 1 },
            instant("2026-11-19T09:00Z"),
        );
        core.tick(instant("2026-11-19T11:00:01Z"), &out)
            .expect("connected");
        assert!(fires.try_recv().is_err(), "two hours old, one-hour window");
    }

    #[test]
    fn after_a_resume_the_reminders_due_during_the_sleep_are_listed() {
        let store = Store::open_in_memory().expect("opens");
        let first = store
            .create_reminder(&draft(at("2026-11-19T10:00"), None))
            .expect("creates");
        let second = store
            .create_reminder(&draft(at("2026-11-19T10:30"), None))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);
        // Asleep from 09:30 to 11:00.
        let woke = instant("2026-11-19T11:00Z");
        core.apply(Command::Recompute(Recompute::Resumed), woke);
        core.tick(woke, &out).expect("connected");
        let listed: Vec<_> = fires
            .try_iter()
            .map(|f| (f.reminder_id, f.missed))
            .collect();
        assert_eq!(listed, vec![(first, true), (second, true)]);
    }

    fn fatigue(trigger: Trigger) -> ReminderDraft {
        ReminderDraft {
            category_id: None,
            title: String::new(),
            body: None,
            authored_lang: "en-US".to_owned(),
            trigger,
            recurrence: None,
        }
    }

    /// Ticks every five minutes from `from` to `to`, as a PC in use would,
    /// and returns what fired.
    fn run_through(
        core: &mut Core,
        out: &SyncSender<Fired>,
        fires: &Receiver<Fired>,
        from: &str,
        to: &str,
    ) -> Vec<(i64, Timestamp)> {
        let (mut now, to) = (instant(from), instant(to));
        let mut fired = Vec::new();
        while now <= to {
            core.tick(now, out).expect("connected");
            fired.extend(drain(fires));
            now = now
                .checked_add(SignedDuration::from_mins(5))
                .expect("in range");
        }
        fired
    }

    #[test]
    fn the_work_hour_limit_counts_across_midnight() {
        // A day begun at 20:00 is not cut at midnight: an 8-hour limit comes
        // at 04:00, once.
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&fatigue(Trigger::WorkHours { minutes: 480 }))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-20T20:00Z"), TimeZone::UTC);
        let fired = run_through(
            &mut core,
            &out,
            &fires,
            "2026-11-20T20:00Z",
            "2026-11-21T06:00Z",
        );
        assert_eq!(fired, vec![(id, instant("2026-11-21T04:00Z"))]);
        assert_eq!(core.workday_start(), instant("2026-11-20T20:00Z"));
        let read = core.store.reminder(id).expect("reads").expect("exists");
        assert_eq!(
            read.status,
            Status::Active,
            "never done: it comes again tomorrow"
        );
    }

    #[test]
    fn an_hour_away_starts_a_new_working_day_and_the_count_again() {
        let store = Store::open_in_memory().expect("opens");
        let limit = store
            .create_reminder(&fatigue(Trigger::WorkHours { minutes: 120 }))
            .expect("creates");
        let eyes = store
            .create_reminder(&fatigue(Trigger::Every { minutes: 20 }))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-20T09:00Z"), TimeZone::UTC);
        let morning = run_through(
            &mut core,
            &out,
            &fires,
            "2026-11-20T09:00Z",
            "2026-11-20T09:40Z",
        );
        assert_eq!(
            morning,
            vec![
                (eyes, instant("2026-11-20T09:20Z")),
                (eyes, instant("2026-11-20T09:40Z")),
            ]
        );

        // Asleep from 09:40 to 13:00; on waking the day starts again, the
        // breaks due while asleep are skipped, not listed as missed, and the
        // limit counts from 13:00.
        core.apply(
            Command::Recompute(Recompute::Resumed),
            instant("2026-11-20T13:00Z"),
        );
        let afternoon = run_through(
            &mut core,
            &out,
            &fires,
            "2026-11-20T13:00Z",
            "2026-11-20T15:00Z",
        );
        assert_eq!(core.workday_start(), instant("2026-11-20T13:00Z"));
        assert!(
            afternoon
                .iter()
                .all(|(_, at)| *at >= instant("2026-11-20T13:20Z"))
        );
        assert!(afternoon.contains(&(limit, instant("2026-11-20T15:00Z"))));
        assert_eq!(
            afternoon.iter().filter(|(id, _)| *id == eyes).count(),
            6,
            "13:20 to 15:00, every 20 minutes"
        );
        assert!(
            core.store.history(eyes).expect("reads").is_empty(),
            "nothing missed"
        );
    }

    #[test]
    fn a_short_sleep_skips_the_break_it_hid_and_keeps_the_day() {
        let store = Store::open_in_memory().expect("opens");
        let eyes = store
            .create_reminder(&fatigue(Trigger::Every { minutes: 20 }))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-20T09:00Z"), TimeZone::UTC);
        core.tick(instant("2026-11-20T09:15Z"), &out)
            .expect("connected");
        // Asleep 09:15 to 09:45: the 09:20 and 09:40 breaks are behind it.
        core.tick(instant("2026-11-20T09:45Z"), &out)
            .expect("connected");
        assert!(fires.try_recv().is_err(), "skipped, not fired late");
        assert_eq!(
            core.workday_start(),
            instant("2026-11-20T09:00Z"),
            "same day"
        );
        assert_eq!(
            core.next().map(|(id, fire)| (id, fire.at)),
            Some((eyes, instant("2026-11-20T10:00Z")))
        );
    }

    #[test]
    fn a_restart_within_the_hour_keeps_the_working_day() {
        let store = Store::open_in_memory().expect("opens");
        let limit = store
            .create_reminder(&fatigue(Trigger::WorkHours { minutes: 480 }))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-20T09:00Z"), TimeZone::UTC);
        // In use all morning: one tick from 09:00 straight to 12:00 would be
        // three hours away, which rightly starts a new day.
        let _ = run_through(
            &mut core,
            &out,
            &fires,
            "2026-11-20T09:00Z",
            "2026-11-20T12:00Z",
        );

        // Quit at 12:00, started again at 12:30: the same day.
        let store = core.into_store();
        let core = Core::new(store, instant("2026-11-20T12:30Z"), TimeZone::UTC);
        assert_eq!(core.workday_start(), instant("2026-11-20T09:00Z"));
        assert_eq!(
            core.next().map(|(id, fire)| (id, fire.at)),
            Some((limit, instant("2026-11-20T17:00Z")))
        );

        // Started again two hours later instead: a new day.
        let store = core.into_store();
        let core = Core::new(store, instant("2026-11-20T14:31Z"), TimeZone::UTC);
        assert_eq!(core.workday_start(), instant("2026-11-20T14:31Z"));
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
    fn a_clock_jumped_forward_lists_the_latest_skipped_and_drops_the_rest() {
        let store = Store::open_in_memory().expect("opens");
        let id = store
            .create_reminder(&draft(at("2026-11-19T10:00"), daily()))
            .expect("creates");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T09:00Z"), TimeZone::UTC);

        // Three days later in one step. Today's 10:00 is two hours old and
        // inside the window, so it is listed; the three before it are not.
        let later = instant("2026-11-22T12:00Z");
        core.apply(Command::Recompute(Recompute::ClockChanged), later);
        core.tick(later, &out).expect("connected");

        let fired = fires.try_recv().expect("listed");
        assert_eq!(fired.reminder_id, id);
        assert_eq!(fired.fire.at, instant("2026-11-22T10:00Z"));
        assert!(fired.missed);
        assert!(fires.try_recv().is_err(), "once, not four times");
        assert_eq!(
            core.store.history(id).expect("reads"),
            vec![(instant("2026-11-21T10:00Z"), NotificationAction::Missed)],
            "the dropped ones are one row, at the newest of them"
        );
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
        let scheduler = Scheduler::spawn(store, out, DEFAULT_MISSED_WINDOW_HOURS).expect("starts");
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

    // ---- prayers (FEAT-0058) ----

    fn prayer_times(day: &str, minutes: [(&str, &str); 3]) -> Command {
        Command::PrayerTimes(
            minutes
                .iter()
                .map(|(prayer, time)| ((*prayer).to_owned(), instant(&format!("{day}T{time}Z"))))
                .collect(),
        )
    }

    fn with_prayers(keys: &[&str], now: &str, tz: TimeZone) -> Core {
        let store = Store::open_in_memory().expect("opens");
        store.sync_prayer_reminders(keys).expect("synced");
        Core::new(store, instant(now), tz)
    }

    fn names(fires: &Receiver<Fired>) -> Vec<(String, Timestamp)> {
        fires
            .try_iter()
            .map(|fired| {
                let key = match fired.reminder.draft.trigger {
                    Trigger::Prayer { prayer } => prayer,
                    _ => "other".to_owned(),
                };
                (key, fired.fire.at)
            })
            .collect()
    }

    #[test]
    fn a_prayer_fires_at_the_time_it_was_given_whatever_the_pcs_zone() {
        // The times are instants - Malaysian clock values already read in
        // Asia/Kuala_Lumpur - so a PC in New York fires them at the same
        // moments a PC in Kuala Lumpur does.
        let new_york = TimeZone::get("America/New_York").expect("a zone");
        let mut core = with_prayers(&["fajr", "dhuhr", "isha"], "2026-11-19T00:00Z", new_york);
        let (out, fires) = fire_channel();
        core.apply(
            prayer_times(
                "2026-11-19",
                [
                    ("fajr", "21:49:00"),
                    ("dhuhr", "05:02:00"),
                    ("isha", "12:12:00"),
                ],
            ),
            instant("2026-11-19T00:00Z"),
        );
        // Nothing early.
        core.tick(instant("2026-11-19T05:01:59Z"), &out)
            .expect("connected");
        assert!(names(&fires).is_empty());
        core.tick(instant("2026-11-19T05:02:00Z"), &out)
            .expect("connected");
        assert_eq!(
            names(&fires),
            [("dhuhr".to_owned(), instant("2026-11-19T05:02:00Z"))]
        );
        core.tick(instant("2026-11-19T12:12:30Z"), &out)
            .expect("connected");
        assert_eq!(
            names(&fires),
            [("isha".to_owned(), instant("2026-11-19T12:12:00Z"))]
        );
    }

    #[test]
    fn a_prayer_is_never_done_and_comes_again_with_the_next_days_times() {
        let mut core = with_prayers(&["fajr"], "2026-11-19T00:00Z", TimeZone::UTC);
        let (out, fires) = fire_channel();
        core.apply(
            Command::PrayerTimes(vec![("fajr".into(), instant("2026-11-19T05:49:00Z"))]),
            instant("2026-11-19T00:00Z"),
        );
        core.tick(instant("2026-11-19T05:49:00Z"), &out)
            .expect("connected");
        assert_eq!(names(&fires).len(), 1);
        // No more times are known, and the reminder is still active, not done.
        let store = core.into_store();
        let row = store.reminders().expect("reads").remove(0);
        assert_eq!(row.status, Status::Active);
        assert_eq!(row.next_fire_at, None);

        // Tomorrow's times arrive: it fires again.
        let mut core = Core::new(store, instant("2026-11-19T06:00Z"), TimeZone::UTC);
        core.apply(
            Command::PrayerTimes(vec![("fajr".into(), instant("2026-11-20T05:49:00Z"))]),
            instant("2026-11-19T06:00Z"),
        );
        core.tick(instant("2026-11-20T05:49:00Z"), &out)
            .expect("connected");
        assert_eq!(
            names(&fires),
            [("fajr".to_owned(), instant("2026-11-20T05:49:00Z"))]
        );
    }

    #[test]
    fn new_times_move_the_fire_and_midnight_and_a_zone_change_keep_it() {
        let mut core = with_prayers(&["dhuhr"], "2026-11-19T00:00Z", TimeZone::UTC);
        core.apply(
            Command::PrayerTimes(vec![("dhuhr".into(), instant("2026-11-19T05:02:00Z"))]),
            instant("2026-11-19T00:00Z"),
        );
        let next = |core: &Core| core.next().map(|(_, fire)| fire.at);
        assert_eq!(next(&core), Some(instant("2026-11-19T05:02:00Z")));

        // A fetch corrected the time, or the zone changed: the fire moves.
        core.apply(
            Command::PrayerTimes(vec![("dhuhr".into(), instant("2026-11-19T05:05:00Z"))]),
            instant("2026-11-19T00:00Z"),
        );
        assert_eq!(next(&core), Some(instant("2026-11-19T05:05:00Z")));

        // The queue is rebuilt at midnight and after a zone change, and the
        // times the app gave are still there.
        core.apply(
            Command::Recompute(Recompute::Midnight),
            instant("2026-11-19T00:00Z"),
        );
        assert_eq!(next(&core), Some(instant("2026-11-19T05:05:00Z")));
        core.set_zone(
            TimeZone::get("Asia/Kuala_Lumpur").expect("a zone"),
            instant("2026-11-19T00:00Z"),
        );
        assert_eq!(next(&core), Some(instant("2026-11-19T05:05:00Z")));
    }

    #[test]
    fn azan_comes_before_an_alarm_before_everything_else_at_the_same_instant() {
        let store = Store::open_in_memory().expect("opens");
        store.sync_prayer_reminders(&["maghrib"]).expect("synced");
        let alarm = store
            .template_for_type("reminder-alarm")
            .expect("reads")
            .expect("seeded");
        // Created so the plain one has the lowest id and would come first by id.
        let plain = store
            .create_reminder(&draft(at("2026-11-19T11:00"), None))
            .expect("creates");
        let mut alarm_draft = draft(at("2026-11-19T11:00"), None);
        alarm_draft.category_id = Some(alarm.id);
        let alarm_id = store.create_reminder(&alarm_draft).expect("creates");
        let mut core = Core::new(store, instant("2026-11-19T10:00Z"), TimeZone::UTC);
        core.apply(
            Command::PrayerTimes(vec![("maghrib".into(), instant("2026-11-19T11:00:00Z"))]),
            instant("2026-11-19T10:00Z"),
        );
        let (out, fires) = fire_channel();
        core.tick(instant("2026-11-19T11:00:00Z"), &out)
            .expect("connected");
        let order: Vec<i64> = fires.try_iter().map(|f| f.reminder_id).collect();
        assert_eq!(order.len(), 3);
        assert_eq!(order[1], alarm_id, "the alarm second");
        assert_eq!(order[2], plain, "the ordinary reminder last");
        assert!(
            order[0] != alarm_id && order[0] != plain,
            "the prayer first"
        );
    }

    #[test]
    fn a_missed_azan_is_listed_inside_the_hour_and_dropped_past_it() {
        for (now, listed) in [
            ("2026-11-19T13:01:00Z", true),
            ("2026-11-19T13:01:00.001Z", false),
        ] {
            // Started with the time stored, as after the app was off.
            let store = Store::open_in_memory().expect("opens");
            store.sync_prayer_reminders(&["dhuhr"]).expect("synced");
            let id = store.reminders().expect("reads")[0].id;
            store
                .set_next_fire_at(id, Some(instant("2026-11-19T12:01:00Z")))
                .expect("stored");
            let (out, fires) = fire_channel();
            let mut core = Core::new(store, instant(now), TimeZone::UTC);
            core.tick(instant(now), &out).expect("connected");
            let fired = fires.try_iter().collect::<Vec<_>>();
            assert_eq!(fired.len(), usize::from(listed), "{now}");
            if let Some(fired) = fired.first() {
                assert!(fired.missed, "listed, not a toast");
            }
            let store = core.into_store();
            let history = store.history(id).expect("reads");
            assert_eq!(
                history
                    .iter()
                    .any(|(_, action)| *action == NotificationAction::Missed),
                !listed,
                "a dropped one leaves a missed row: {now}"
            );
        }
    }

    #[test]
    fn the_azan_window_is_configurable() {
        let store = Store::open_in_memory().expect("opens");
        store.sync_prayer_reminders(&["asr"]).expect("synced");
        let id = store.reminders().expect("reads")[0].id;
        store
            .set_next_fire_at(id, Some(instant("2026-11-19T08:00:00Z")))
            .expect("stored");
        let (out, fires) = fire_channel();
        let mut core = Core::new(store, instant("2026-11-19T08:45:00Z"), TimeZone::UTC);
        // 45 minutes late, with a 30-minute window: dropped.
        core.apply(
            Command::SetAzanWindow { minutes: 30 },
            instant("2026-11-19T08:45:00Z"),
        );
        core.tick(instant("2026-11-19T08:45:00Z"), &out)
            .expect("connected");
        assert_eq!(fires.try_iter().count(), 0);
    }

    #[test]
    fn enabling_prayers_makes_their_rows_and_queues_them() {
        let store = Store::open_in_memory().expect("opens");
        let mut core = Core::new(store, instant("2026-11-19T00:00Z"), TimeZone::UTC);
        core.apply(
            Command::PrayerTimes(vec![
                ("fajr".into(), instant("2026-11-19T05:40:00Z")),
                ("syuruk".into(), instant("2026-11-19T06:00:00Z")),
            ]),
            instant("2026-11-19T00:00Z"),
        );
        assert!(core.next().is_none(), "no prayer is enabled yet");
        core.apply(
            Command::EnablePrayers(vec!["fajr".into()]),
            instant("2026-11-19T00:00Z"),
        );
        assert_eq!(
            core.next().map(|(_, fire)| fire.at),
            Some(instant("2026-11-19T05:40:00Z"))
        );
        core.apply(
            Command::EnablePrayers(vec!["fajr".into(), "syuruk".into()]),
            instant("2026-11-19T00:00Z"),
        );
        let (out, fires) = fire_channel();
        core.tick(instant("2026-11-19T06:00:30Z"), &out)
            .expect("connected");
        let fired = names(&fires);
        assert_eq!(
            fired.len(),
            2,
            "Fajr was due too, inside the hour: {fired:?}"
        );
    }

    #[test]
    fn jumuah_is_a_prayer_like_the_others_with_its_own_times() {
        let mut core = with_prayers(&["dhuhr", "jumuah"], "2026-11-19T00:00Z", TimeZone::UTC);
        core.apply(
            Command::PrayerTimes(vec![
                ("dhuhr".into(), instant("2026-11-19T05:02:00Z")),
                ("jumuah".into(), instant("2026-11-20T05:02:00Z")),
            ]),
            instant("2026-11-19T00:00Z"),
        );
        let (out, fires) = fire_channel();
        core.tick(instant("2026-11-19T05:02:00Z"), &out)
            .expect("connected");
        core.tick(instant("2026-11-20T05:02:00Z"), &out)
            .expect("connected");
        let fired: Vec<String> = fires
            .try_iter()
            .filter_map(|f| match f.reminder.draft.trigger {
                Trigger::Prayer { prayer } => Some(prayer),
                _ => None,
            })
            .collect();
        assert_eq!(fired, ["dhuhr", "jumuah"]);
    }

    // ---- pre-prayer reminders (FEAT-0061) ----

    fn pre_core(keys: &[&str], minutes: u32, times: &[(&str, &str)], now: &str) -> Core {
        let store = Store::open_in_memory().expect("opens");
        let mut core = Core::new(store, instant(now), TimeZone::UTC);
        core.apply(
            Command::PrayerTimes(
                times
                    .iter()
                    .map(|(prayer, time)| ((*prayer).to_owned(), instant(time)))
                    .collect(),
            ),
            instant(now),
        );
        core.apply(
            Command::EnablePrePrayers {
                keys: keys.iter().map(|k| (*k).to_owned()).collect(),
                minutes,
            },
            instant(now),
        );
        core
    }

    #[test]
    fn a_pre_prayer_reminder_comes_its_lead_before_the_prayer_and_says_so() {
        let mut core = pre_core(
            &["dhuhr"],
            10,
            &[("dhuhr", "2026-11-19T05:02:00Z")],
            "2026-11-19T00:00Z",
        );
        let (out, fires) = fire_channel();
        core.tick(instant("2026-11-19T04:51:59Z"), &out)
            .expect("connected");
        assert!(fires.try_iter().next().is_none(), "nothing early");
        core.tick(instant("2026-11-19T04:52:00Z"), &out)
            .expect("connected");
        let fired: Vec<_> = fires.try_iter().collect();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].fire.at, instant("2026-11-19T04:52:00Z"));
        assert_eq!(fired[0].fire.occurrence, instant("2026-11-19T05:02:00Z"));
        assert_eq!(
            fired[0].fire.offset,
            Some(ot_store::reminders::Offset {
                amount: 10,
                unit: ot_store::reminders::OffsetUnit::Minutes
            })
        );
        assert!(fired[0].reminder.draft.trigger.is_pre_prayer());
        // Never done: it stays active for the next day's times.
        let store = core.into_store();
        let row = store
            .reminders()
            .expect("reads")
            .into_iter()
            .find(|r| r.draft.trigger.is_pre_prayer())
            .expect("the row stays");
        assert_eq!(row.status, Status::Active);
    }

    #[test]
    fn a_pre_prayer_reminder_moves_when_the_prayer_does_and_when_the_lead_changes() {
        let mut core = pre_core(
            &["dhuhr"],
            10,
            &[("dhuhr", "2026-11-19T05:02:00Z")],
            "2026-11-19T00:00Z",
        );
        let next = |core: &Core| core.next().map(|(_, fire)| fire.at);
        assert_eq!(next(&core), Some(instant("2026-11-19T04:52:00Z")));
        // A corrected time for the prayer moves its reminder with it.
        core.apply(
            Command::PrayerTimes(vec![("dhuhr".into(), instant("2026-11-19T05:20:00Z"))]),
            instant("2026-11-19T00:00Z"),
        );
        assert_eq!(next(&core), Some(instant("2026-11-19T05:10:00Z")));
        // A new lead moves it too, in the same row.
        core.apply(
            Command::EnablePrePrayers {
                keys: vec!["dhuhr".into()],
                minutes: 30,
            },
            instant("2026-11-19T00:00Z"),
        );
        assert_eq!(next(&core), Some(instant("2026-11-19T04:50:00Z")));
        // And turning it off removes it.
        core.apply(
            Command::EnablePrePrayers {
                keys: Vec::new(),
                minutes: 30,
            },
            instant("2026-11-19T00:00Z"),
        );
        assert_eq!(next(&core), None);
    }

    #[test]
    fn a_pre_prayer_reminder_that_comes_due_late_is_skipped_not_listed() {
        let mut core = pre_core(
            &["dhuhr"],
            10,
            &[("dhuhr", "2026-11-19T05:02:00Z")],
            "2026-11-19T00:00Z",
        );
        let (out, fires) = fire_channel();
        // The PC slept through it and woke with the prayer already here.
        core.tick(instant("2026-11-19T05:05:00Z"), &out)
            .expect("connected");
        assert!(
            fires.try_iter().next().is_none(),
            "neither fired nor listed as missed"
        );
    }
}
