//! The prayer service: the work that keeps the day's times current when
//! Settings is closed (FEAT-0058, `docs/11-prayer-and-azan.md` §4, §6, §9).
//!
//! One thread of its own. A pass of [`Engine::run`] reads the settings and the
//! saved place, looks the place up again if it is a day old and the user
//! agreed, fetches from e-Solat when a fetch is due, works out today's and
//! tomorrow's times through the fallback chain, and gives them to the
//! scheduler. The thread then waits, with a bounded timeout and never a long
//! sleep, until the next thing is due, or until Settings says something
//! changed.
//!
//! **When a fetch is due** is the whole policy, and it is made of
//! `ot_prayer::fetch`'s pure schedule: the cache has no row for today (which is
//! also what a zone change looks like), or the stored time of the next check
//! has come — weekly, or daily while the answer stopped short at the year's end.
//! After a failure the next attempt is one, five and fifteen minutes away, then
//! the next check. There is no polling loop: nothing runs until the next
//! thing is due.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender, channel, sync_channel};
use std::time::Duration;

use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use ot_core::{DataRoot, Settings};
use ot_prayer::chain::{Cache, Fetch, Resolution, Source, SourceChoice, should_toast};
use ot_prayer::fetch::{self, Cancel, FetchError, Fetched, Range};
use ot_prayer::jakim::Prayer;
use ot_prayer::location::{
    self, Consent, DETECTION_TTL, Located, LookupError, Origin as LocationOrigin, Step,
};
use ot_sched::scheduler::Command;
use ot_store::Store;
use ot_store::location::Origin;

use crate::prayer::{self, Plan, StoreCache};

/// The longest the thread waits before it looks at the clock again, however
/// far off the next thing is: a PC that slept must not leave it holding a stale
/// deadline (`.claude/rules/threads-and-scheduling.md`).
const MAX_WAIT: Duration = Duration::from_secs(30);

/// The `settings` keys the service's own memory is kept under, so it survives a
/// restart: when the next fetch is due, and the day an error was last shown.
const NEXT_FETCH: &str = "prayer.next_fetch";
const LAST_ERROR_TOAST: &str = "prayer.last_error_toast";

/// How long before a failed IP lookup is tried again.
const DETECT_RETRY: SignedDuration = SignedDuration::from_hours(1);

/// What the service tells the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Note {
    /// No source could give today's times — shown once a day.
    Unavailable,
    /// Where today's times came from, for the label on a prayer's toast.
    Source(Source),
}

/// What the window tells the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Msg {
    /// The settings or the saved place changed: work the day out again now.
    Changed,
    Stop,
}

/// What one pass found, for the thread that runs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    /// When the next pass is due.
    pub next_run: Timestamp,
    /// Whether the user should be told that no times are available.
    pub unavailable: bool,
    /// Where today's times came from, when there are any.
    pub source: Option<Source>,
}

/// A fetch that is never made: the chain run for the day's times must not fetch,
/// because when to fetch is this service's decision alone.
struct NoFetch;

impl Fetch for NoFetch {
    fn fetch(&self, _: &str, _: Range, _: &Cancel) -> Result<Fetched, FetchError> {
        Err(FetchError::Network("not asked for here".to_owned()))
    }
}

type Detect = Box<dyn Fn(Consent) -> Result<Located, LookupError> + Send>;

/// One pass of the work, with the network and the scheduler as inputs, so the
/// policy can be tested without either.
pub struct Engine {
    root: PathBuf,
    scheduler: Sender<Command>,
    fetch: Box<dyn Fetch + Send>,
    detect: Detect,
    cancel: Cancel,
    /// What the scheduler was last told, so it is told again only on a change.
    sent_enabled: Option<Vec<String>>,
    sent_window: Option<u32>,
    /// Failures of the fetch in a row, and when to try again.
    failures: u32,
    retry_at: Option<Timestamp>,
    /// When a failed IP lookup may be tried again.
    detect_retry_at: Option<Timestamp>,
}

/// A prayer's key in the scheduler, or `None` for the two that have no
/// notification (Imsak and Dhuha).
fn key_of(prayer: Prayer) -> Option<&'static str> {
    Some(match prayer {
        Prayer::Fajr => "fajr",
        Prayer::Syuruk => "syuruk",
        Prayer::Dhuhr => "dhuhr",
        Prayer::Asr => "asr",
        Prayer::Maghrib => "maghrib",
        Prayer::Isha => "isha",
        _ => return None,
    })
}

/// The next local midnight in `tz`, or a day from now if the zone cannot say.
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

impl Engine {
    pub fn new(
        root: &Path,
        scheduler: Sender<Command>,
        fetch: Box<dyn Fetch + Send>,
        detect: Detect,
        cancel: Cancel,
    ) -> Self {
        Self {
            root: root.to_path_buf(),
            scheduler,
            fetch,
            detect,
            cancel,
            sent_enabled: None,
            sent_window: None,
            failures: 0,
            retry_at: None,
            detect_retry_at: None,
        }
    }

    fn send(&self, command: Command) {
        if self.scheduler.send(command).is_err() {
            tracing::warn!("the scheduler has stopped; a prayer command was lost");
        }
    }

    fn settings(&self) -> Settings {
        let loaded = DataRoot::resolve(Some(&self.root))
            .map_err(|error| error.to_string())
            .and_then(|root| Settings::load(&root).map_err(|error| error.to_string()));
        loaded.unwrap_or_else(|reason| {
            tracing::warn!(%reason, "the settings could not be read; using the defaults");
            Settings::default()
        })
    }

    /// One pass. Never panics and never blocks for longer than a fetch's own
    /// timeout; whatever fails is logged and the next pass carries on.
    pub fn run(&mut self, now: Timestamp) -> Report {
        let settings = self.settings();
        let store = match Store::open(&self.root) {
            Ok(store) => store,
            Err(error) => {
                tracing::warn!(%error, "the prayer service could not open the store");
                return Report {
                    next_run: now + SignedDuration::from_mins(1),
                    unavailable: false,
                    source: None,
                };
            }
        };

        // Which prayers have a notification, and how late a missed one may be.
        let enabled: Vec<String> = settings
            .prayer
            .enabled_prayers()
            .iter()
            .map(|key| (*key).to_owned())
            .collect();
        if self.sent_enabled.as_ref() != Some(&enabled) {
            self.send(Command::EnablePrayers(enabled.clone()));
            self.sent_enabled = Some(enabled.clone());
        }
        let window = settings.prayer.azan_window();
        if self.sent_window != Some(window) {
            self.send(Command::SetAzanWindow { minutes: window });
            self.sent_window = Some(window);
        }

        // The place, looked up again after a day when the user agreed.
        let consent = Consent::from(settings.location.ip_consent);
        let mut saved = store.location().ok().flatten();
        let step = location::next_step(consent, saved_for_rule(saved.as_ref()), now);
        if step == Step::Lookup && self.detect_retry_at.is_none_or(|at| now >= at) {
            match (self.detect)(Consent::Granted) {
                Ok(found) => {
                    let place = prayer::stored(&found, Origin::Ip, now);
                    if let Err(error) = store.save_detected_location(&place) {
                        tracing::warn!(%error, "a detected place could not be saved");
                    }
                    saved = store.location().ok().flatten();
                    self.detect_retry_at = None;
                    tracing::info!("the place was looked up again");
                }
                Err(error) => {
                    tracing::warn!(%error, "the place could not be looked up");
                    self.detect_retry_at = Some(now + DETECT_RETRY);
                }
            }
        }

        let plan = prayer::plan(&settings.prayer, saved.as_ref());
        let today = prayer::today(&plan, now);
        let wants_jakim = match plan.source {
            SourceChoice::Jakim => true,
            SourceChoice::Auto => plan.in_malaysia,
            SourceChoice::Calculated => false,
        };
        if wants_jakim
            && let Some(zone) = plan.zone
            && self.fetch_due(&store, &zone.code, today, now)
        {
            self.fetch_now(&store, &zone.code, today, now);
        }

        // Today's and tomorrow's times, through the chain with no fetch of its own.
        let mut times: Vec<(String, Timestamp)> = Vec::new();
        let mut any = false;
        let mut source = None;
        for days_ahead in 0..2 {
            let at = now + SignedDuration::from_hours(24 * days_ahead);
            if let Resolution::Times(resolved) =
                prayer::resolve_day(&store, &NoFetch, &plan, at, &self.cancel)
            {
                any = true;
                source.get_or_insert(resolved.source);
                times.extend(resolved.times.into_iter().filter_map(|(prayer, at)| {
                    let key = key_of(prayer)?;
                    enabled
                        .iter()
                        .any(|e| e == key)
                        .then(|| (key.to_owned(), at))
                }));
            }
        }
        // Nothing resolved keeps the last times the scheduler has: they are
        // better than none (spec §7, "keep the last known times").
        if any {
            self.send(Command::PrayerTimes(times));
        }
        let unavailable = !any && self.may_say_unavailable(&store, today);

        Report {
            next_run: self.next_run(&store, &plan, wants_jakim, saved.as_ref(), consent, now),
            unavailable,
            source,
        }
    }

    /// Whether a fetch is due: the cache has nothing for today, or the stored
    /// time of the next check has come — and not while a retry is waiting.
    fn fetch_due(
        &self,
        store: &Store,
        zone: &str,
        today: jiff::civil::Date,
        now: Timestamp,
    ) -> bool {
        if self.retry_at.is_some_and(|at| now < at) {
            return false;
        }
        if StoreCache(store).day(zone, today).is_none() {
            return true;
        }
        read_stamp(store, NEXT_FETCH).is_none_or(|at| at <= now)
    }

    fn fetch_now(&mut self, store: &Store, zone: &str, today: jiff::civil::Date, now: Timestamp) {
        match self.fetch.fetch(zone, fetch::range(today), &self.cancel) {
            Ok(fetched) => {
                if self.cancel.is_cancelled() {
                    return;
                }
                StoreCache(store).store(zone, &fetched.days, now);
                let after = fetch::next_after_success(&fetched);
                self.failures = 0;
                self.retry_at = None;
                if let Err(error) = store.set_setting(
                    NEXT_FETCH,
                    &ot_store::reminders::stamp(
                        now + SignedDuration::try_from(after).unwrap_or_default(),
                    ),
                ) {
                    tracing::warn!(%error, "the next fetch time could not be stored");
                }
                tracing::info!(
                    zone,
                    days = fetched.days.len(),
                    cut_short = fetched.cut_short(),
                    next_in_hours = after.as_secs() / 3600,
                    "fetched prayer times"
                );
            }
            Err(error) => {
                self.failures += 1;
                let wait = fetch::next_after_failure(&error, self.failures)
                    .and_then(|wait| SignedDuration::try_from(wait).ok())
                    .unwrap_or(SignedDuration::from_hours(24 * 7));
                self.retry_at = Some(now + wait);
                tracing::warn!(
                    zone,
                    %error,
                    failures = self.failures,
                    retry_in_secs = wait.as_secs(),
                    "the prayer fetch failed"
                );
            }
        }
    }

    /// Once a day, however often a pass finds nothing.
    fn may_say_unavailable(&self, store: &Store, today: jiff::civil::Date) -> bool {
        let last = store
            .setting(LAST_ERROR_TOAST)
            .ok()
            .flatten()
            .and_then(|text| text.parse::<jiff::civil::Date>().ok());
        if !should_toast(last, today) {
            return false;
        }
        if let Err(error) = store.set_setting(LAST_ERROR_TOAST, &today.to_string()) {
            tracing::warn!(%error, "the day of the last error could not be stored");
        }
        true
    }

    /// The soonest thing due: midnight in the place's zone, a retry, the next
    /// check, a daily look-up of the place.
    fn next_run(
        &self,
        store: &Store,
        plan: &Plan,
        wants_jakim: bool,
        saved: Option<&ot_store::location::StoredLocation>,
        consent: Consent,
        now: Timestamp,
    ) -> Timestamp {
        let tz = TimeZone::get(&plan.tz).unwrap_or(TimeZone::UTC);
        // A few seconds past midnight, so "today" has certainly turned over.
        let mut next = next_midnight(now, &tz) + SignedDuration::from_secs(5);
        let mut consider = |at: Option<Timestamp>| {
            if let Some(at) = at {
                next = next.min(at);
            }
        };
        if wants_jakim {
            consider(self.retry_at);
            consider(read_stamp(store, NEXT_FETCH));
        }
        if consent == Consent::Granted
            && let Some(place) = saved
            && place.origin == Origin::Ip
        {
            consider(Some(
                self.detect_retry_at
                    .unwrap_or(place.updated_at + DETECTION_TTL),
            ));
        }
        // Never in the past or the very next instant: a pass that finds nothing
        // due must not be run again at once.
        next.max(now + SignedDuration::from_secs(1))
    }
}

fn saved_for_rule(
    place: Option<&ot_store::location::StoredLocation>,
) -> Option<(LocationOrigin, Timestamp)> {
    place.map(|place| {
        let origin = match place.origin {
            Origin::Ip => LocationOrigin::Ip,
            Origin::Manual => LocationOrigin::Manual,
        };
        (origin, place.updated_at)
    })
}

fn read_stamp(store: &Store, key: &str) -> Option<Timestamp> {
    store
        .setting(key)
        .ok()
        .flatten()
        .and_then(|text| text.parse().ok())
}

/// The thread, and the window's hold on it.
pub struct Service {
    control: Sender<Msg>,
    notes: Receiver<Note>,
    cancel: Cancel,
}

impl Service {
    /// Start the service on `root`'s store, talking to `scheduler`.
    pub fn start(root: &Path, scheduler: Sender<Command>) -> Self {
        let (control, incoming) = channel();
        let (note_out, notes) = sync_channel(4);
        let cancel = Cancel::new();
        let engine = Engine::new(
            root,
            scheduler,
            Box::new(fetch::Client::new()),
            Box::new(|consent| location::Lookup::new().detect(consent)),
            cancel.clone(),
        );
        if let Err(error) = std::thread::Builder::new()
            .name("prayer-service".into())
            .spawn(move || serve(engine, &incoming, &note_out))
        {
            tracing::error!(%error, "the prayer service could not be started");
        }
        Self {
            control,
            notes,
            cancel,
        }
    }

    /// The settings or the saved place changed.
    pub fn changed(&self) {
        let _ = self.control.send(Msg::Changed);
    }

    /// What the service has to say.
    pub fn drain(&self) -> Vec<Note> {
        self.notes.try_iter().collect()
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        // The window is closing for good: stop the work in flight, and the thread.
        self.cancel.cancel();
        let _ = self.control.send(Msg::Stop);
    }
}

fn serve(mut engine: Engine, incoming: &Receiver<Msg>, notes: &SyncSender<Note>) {
    let mut next = Timestamp::now();
    let mut due = true;
    loop {
        let now = Timestamp::now();
        if due || now >= next {
            let report = engine.run(now);
            next = report.next_run;
            due = false;
            if report.unavailable {
                let _ = notes.try_send(Note::Unavailable);
            }
            if let Some(source) = report.source {
                let _ = notes.try_send(Note::Source(source));
            }
        }
        let wait = Duration::try_from(next.duration_since(Timestamp::now()))
            .unwrap_or(Duration::ZERO)
            .clamp(Duration::from_secs(1), MAX_WAIT);
        match incoming.recv_timeout(wait) {
            Ok(Msg::Changed) => {
                // Several changes in a row are one pass.
                while let Ok(Msg::Changed) = incoming.try_recv() {}
                due = true;
            }
            Ok(Msg::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ot_prayer::jakim::{self, PrayerDay, Response};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};

    const SGR01: &str =
        include_str!("../../ot-prayer/tests/fixtures/esolat-2026-11-20/SGR01-month.json");

    fn at(text: &str) -> Timestamp {
        text.parse().expect("an instant")
    }

    fn november() -> Vec<PrayerDay> {
        match jakim::parse(SGR01.as_bytes()).expect("parses") {
            Response::Days { days, .. } => days,
            Response::NoRecord => panic!("no record"),
        }
    }

    /// A fetch that answers what it is told and counts its calls.
    #[derive(Clone)]
    struct StubFetch {
        answer: Arc<Mutex<Result<Vec<PrayerDay>, FetchError>>>,
        calls: Arc<AtomicU32>,
    }

    impl StubFetch {
        fn new(answer: Result<Vec<PrayerDay>, FetchError>) -> Self {
            Self {
                answer: Arc::new(Mutex::new(answer)),
                calls: Arc::new(AtomicU32::new(0)),
            }
        }
        fn calls(&self) -> u32 {
            self.calls.load(Ordering::SeqCst)
        }
        fn answer(&self, answer: Result<Vec<PrayerDay>, FetchError>) {
            *self.answer.lock().expect("lock") = answer;
        }
    }

    impl Fetch for StubFetch {
        fn fetch(&self, zone: &str, range: Range, _: &Cancel) -> Result<Fetched, FetchError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.answer
                .lock()
                .expect("lock")
                .clone()
                .map(|days| Fetched {
                    zone: zone.to_owned(),
                    days,
                    rejected: Vec::new(),
                    requested: range,
                })
        }
    }

    struct Rig {
        dir: tempfile::TempDir,
        engine: Engine,
        commands: Receiver<Command>,
        fetch: StubFetch,
        detections: Arc<AtomicU32>,
    }

    fn rig(settings: &Settings, answer: Result<Vec<PrayerDay>, FetchError>) -> Rig {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let root = DataRoot::resolve(Some(dir.path())).expect("resolves");
        settings.save(&root).expect("saved");
        // As `main` does: the database exists before any thread uses it.
        drop(Store::open(dir.path()).expect("created"));
        let (scheduler, commands) = channel();
        let fetch = StubFetch::new(answer);
        let detections = Arc::new(AtomicU32::new(0));
        let counted = detections.clone();
        let engine = Engine::new(
            dir.path(),
            scheduler,
            Box::new(fetch.clone()),
            Box::new(move |_| {
                counted.fetch_add(1, Ordering::SeqCst);
                Ok(Located {
                    name: "Shah Alam, Selangor".to_owned(),
                    latitude: 3.09,
                    longitude: 101.53,
                    tz: "Asia/Kuala_Lumpur".to_owned(),
                    country_code: Some("MY".to_owned()),
                })
            }),
            Cancel::new(),
        );
        Rig {
            dir,
            engine,
            commands,
            fetch,
            detections,
        }
    }

    impl Rig {
        fn store(&self) -> Store {
            Store::open(self.dir.path()).expect("opens")
        }

        fn place_in_malaysia(&self, origin: Origin, updated: &str) {
            let place = ot_store::location::StoredLocation {
                name: "Shah Alam".to_owned(),
                latitude: 3.09,
                longitude: 101.53,
                tz: "Asia/Kuala_Lumpur".to_owned(),
                country_code: Some("MY".to_owned()),
                origin,
                updated_at: at(updated),
            };
            match origin {
                Origin::Manual => self.store().save_manual_location(&place).expect("saved"),
                Origin::Ip => {
                    self.store().save_detected_location(&place).expect("saved");
                }
            }
        }

        fn commands(&self) -> Vec<Command> {
            self.commands.try_iter().collect()
        }
    }

    fn prayer_times(commands: &[Command]) -> Vec<(String, Timestamp)> {
        commands
            .iter()
            .filter_map(|c| match c {
                Command::PrayerTimes(times) => Some(times.clone()),
                _ => None,
            })
            .next_back()
            .unwrap_or_default()
    }

    const NOW: &str = "2026-11-20T01:00:00Z";

    #[test]
    fn a_first_pass_enables_the_prayers_fetches_and_gives_the_scheduler_two_days() {
        let mut rig = rig(&Settings::default(), Ok(november()));
        rig.place_in_malaysia(Origin::Manual, "2026-11-19T00:00:00Z");
        let report = rig.engine.run(at(NOW));
        assert_eq!(rig.fetch.calls(), 1, "the cache had nothing for today");

        let commands = rig.commands();
        assert!(
            commands.contains(&Command::EnablePrayers(
                ["fajr", "dhuhr", "asr", "maghrib", "isha"]
                    .map(str::to_owned)
                    .to_vec()
            ))
        );
        assert!(commands.contains(&Command::SetAzanWindow { minutes: 60 }));
        let times = prayer_times(&commands);
        assert_eq!(
            times.len(),
            10,
            "five prayers, today and tomorrow: {times:?}"
        );
        // Fajr on the 20th at 05:49 in Kuala Lumpur is 21:49 UTC the evening
        // before: JAKIM's clock value read in its own zone.
        assert!(times.contains(&("fajr".to_owned(), at("2026-11-19T21:49:00Z"))));
        assert!(!report.unavailable);
        // The answer stopped short of the range, so the next check is daily.
        assert!(report.next_run <= at(NOW) + SignedDuration::from_hours(24));
    }

    #[test]
    fn a_second_pass_the_same_day_does_not_fetch_again_but_still_gives_the_times() {
        let mut rig = rig(&Settings::default(), Ok(november()));
        rig.place_in_malaysia(Origin::Manual, "2026-11-19T00:00:00Z");
        rig.engine.run(at(NOW));
        rig.commands();
        rig.engine.run(at("2026-11-20T02:00:00Z"));
        assert_eq!(rig.fetch.calls(), 1, "a fetch was not due");
        let commands = rig.commands();
        assert_eq!(prayer_times(&commands).len(), 10);
        assert!(
            !commands
                .iter()
                .any(|c| matches!(c, Command::EnablePrayers(_))),
            "the scheduler is told once, not on every pass"
        );
    }

    #[test]
    fn the_next_check_comes_when_it_is_due() {
        let mut rig = rig(&Settings::default(), Ok(november()));
        rig.place_in_malaysia(Origin::Manual, "2026-11-19T00:00:00Z");
        rig.engine.run(at(NOW));
        // A day later the daily check (the answer stopped short) is due.
        rig.engine.run(at("2026-11-21T02:00:00Z"));
        assert_eq!(rig.fetch.calls(), 2);
    }

    #[test]
    fn a_failure_retries_after_one_then_five_then_fifteen_minutes_and_not_before() {
        let mut rig = rig(
            &Settings::default(),
            Err(FetchError::Network("down".into())),
        );
        rig.place_in_malaysia(Origin::Manual, "2026-11-19T00:00:00Z");
        let start = at(NOW);
        let report = rig.engine.run(start);
        assert_eq!(rig.fetch.calls(), 1);
        // The times still come, calculated, so the day is not left without.
        assert_eq!(prayer_times(&rig.commands()).len(), 10, "the fallback");
        assert_eq!(report.next_run, start + SignedDuration::from_mins(1));

        // Before the minute is up nothing is asked.
        rig.engine.run(start + SignedDuration::from_secs(59));
        assert_eq!(rig.fetch.calls(), 1);
        // At the minute, a second attempt; its failure waits five.
        let second = start + SignedDuration::from_mins(1);
        let report = rig.engine.run(second);
        assert_eq!(rig.fetch.calls(), 2);
        assert_eq!(report.next_run, second + SignedDuration::from_mins(5));
        let third = second + SignedDuration::from_mins(5);
        let report = rig.engine.run(third);
        assert_eq!(rig.fetch.calls(), 3);
        assert_eq!(report.next_run, third + SignedDuration::from_mins(15));

        // The network comes back: the next attempt succeeds and the schedule resets.
        rig.fetch.answer(Ok(november()));
        rig.engine.run(third + SignedDuration::from_mins(15));
        assert_eq!(rig.fetch.calls(), 4);
        let times = prayer_times(&rig.commands());
        assert!(
            times.contains(&("fajr".to_owned(), at("2026-11-19T21:49:00Z"))),
            "JAKIM's own time is back"
        );
    }

    #[test]
    fn a_zone_change_fetches_for_the_new_zone() {
        let mut rig = rig(&Settings::default(), Ok(november()));
        rig.place_in_malaysia(Origin::Manual, "2026-11-19T00:00:00Z");
        rig.engine.run(at(NOW));
        assert_eq!(rig.fetch.calls(), 1);
        // The user chooses another zone: the cache has nothing for it today.
        let mut settings = Settings::default();
        settings.prayer.zone = Some("WLY01".to_owned());
        settings
            .save(&DataRoot::resolve(Some(rig.dir.path())).expect("resolves"))
            .expect("saved");
        rig.engine.run(at("2026-11-20T01:05:00Z"));
        assert_eq!(rig.fetch.calls(), 2, "a zone change is a fetch");
    }

    #[test]
    fn calculated_by_choice_never_fetches() {
        let mut settings = Settings::default();
        settings.prayer.source = "calculated".to_owned();
        let mut rig = rig(&settings, Ok(november()));
        rig.place_in_malaysia(Origin::Manual, "2026-11-19T00:00:00Z");
        rig.engine.run(at(NOW));
        assert_eq!(rig.fetch.calls(), 0);
        assert_eq!(prayer_times(&rig.commands()).len(), 10);
    }

    #[test]
    fn with_no_place_and_no_zone_the_user_is_told_once_a_day() {
        let mut rig = rig(&Settings::default(), Ok(november()));
        let first = rig.engine.run(at(NOW));
        assert!(first.unavailable, "nothing to go on: say so");
        assert!(
            prayer_times(&rig.commands()).is_empty(),
            "and tell the scheduler nothing"
        );
        let again = rig.engine.run(at("2026-11-20T03:00:00Z"));
        assert!(!again.unavailable, "once a day, not every pass");
        let tomorrow = rig.engine.run(at("2026-11-21T03:00:00Z"));
        assert!(tomorrow.unavailable, "and again the next day");
    }

    #[test]
    fn a_detected_place_is_looked_up_again_after_a_day_with_consent_and_a_manual_one_never() {
        let mut settings = Settings::default();
        settings.location.ip_consent = Some(true);
        let mut rig = rig(&settings, Ok(november()));
        rig.place_in_malaysia(Origin::Ip, "2026-11-18T00:00:00Z");
        rig.engine.run(at(NOW));
        assert_eq!(rig.detections.load(Ordering::SeqCst), 1, "older than a day");

        // Fresh now: not looked up on the next pass.
        rig.engine.run(at("2026-11-20T02:00:00Z"));
        assert_eq!(rig.detections.load(Ordering::SeqCst), 1);

        // A place the user chose is never looked up, however old.
        let mut manual = self::rig(&settings, Ok(november()));
        manual.place_in_malaysia(Origin::Manual, "2026-10-01T00:00:00Z");
        manual.engine.run(at(NOW));
        assert_eq!(manual.detections.load(Ordering::SeqCst), 0);

        // And without consent nothing is looked up at all.
        let mut declined = self::rig(&Settings::default(), Ok(november()));
        declined.place_in_malaysia(Origin::Ip, "2026-10-01T00:00:00Z");
        declined.engine.run(at(NOW));
        assert_eq!(declined.detections.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn syuruk_is_told_to_the_scheduler_only_when_it_is_switched_on() {
        let mut settings = Settings::default();
        settings.prayer.notify_syuruk = true;
        settings.prayer.azan_window_minutes = 30;
        let mut rig = rig(&settings, Ok(november()));
        rig.place_in_malaysia(Origin::Manual, "2026-11-19T00:00:00Z");
        rig.engine.run(at(NOW));
        let commands = rig.commands();
        assert!(commands.contains(&Command::SetAzanWindow { minutes: 30 }));
        let times = prayer_times(&commands);
        assert_eq!(times.len(), 12, "six prayers, two days");
        assert!(times.iter().any(|(key, _)| key == "syuruk"));
        assert!(
            !times
                .iter()
                .any(|(key, _)| key == "imsak" || key == "dhuha")
        );
    }
}
