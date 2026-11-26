//! The weather service (FEAT-0069, `docs/specs/weather.md` §3, §4 and §7).
//!
//! A thread that polls Open-Meteo for the saved place, keeps the last answer in
//! the cache, feeds each fresh answer to the change detector and tells the window
//! what changed. Like the prayer service it is an [`Engine`] of one testable pass
//! and a thread that runs it, so the policy can be tested without a network or a
//! clock.
//!
//! - **Off means no requests.** With weather off nothing is fetched and the thread
//!   only waits to be told the settings changed.
//! - **The cache is the offline answer.** A failed fetch keeps the cached data,
//!   judged stale by its age, and never feeds the detector: only an answer fetched
//!   a moment ago can be a change.
//! - **Retries back off** (1, 5 and 15 minutes), and there is no polling loop
//!   tighter than the configured interval.
//! - **Cancel stops the work**: the window dropping the service cancels the one in
//!   flight, and the thread ends.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender, channel, sync_channel};
use std::time::Duration;

use jiff::{SignedDuration, Timestamp};
use ot_core::{DataRoot, Settings};
use ot_store::Store;
use ot_store::weather::DEFAULT_TTL;
use ot_weather::condition::{Condition, classify};
use ot_weather::detect::{Change, Detector, Direction, Threshold};
use ot_weather::fetch::{self, Cancel, FetchError};
use ot_weather::observation::{Observation, parse};

/// The longest the thread sleeps before looking at the clock again, so a laptop
/// that slept wakes to the right time rather than to an eight-hour wait.
const MAX_WAIT: Duration = Duration::from_secs(30);

/// Where the last class the user was told about is kept.
const NOTIFIED: &str = "weather.notified";

/// How long to wait when weather is off, no place is saved or the store cannot be
/// opened. A change of settings or place wakes the thread sooner.
const IDLE: SignedDuration = SignedDuration::from_mins(10);

/// What a fetch is: a place in, an observation and the body it was read from out.
pub trait Fetch: Send {
    fn fetch(
        &self,
        latitude: f64,
        longitude: f64,
        cancel: &Cancel,
    ) -> Result<(Observation, String), FetchError>;
}

impl Fetch for fetch::Client {
    fn fetch(
        &self,
        latitude: f64,
        longitude: f64,
        cancel: &Cancel,
    ) -> Result<(Observation, String), FetchError> {
        self.fetch_with_body(latitude, longitude, cancel)
    }
}

/// The weather as the window may show it (`docs/specs/weather.md` §7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapshot {
    pub condition: Condition,
    pub temperature_c: f64,
    pub fetched_at: Timestamp,
    /// Older than its TTL when it was made: shown marked as old, and never compared.
    pub stale: bool,
    /// How old it may be before it counts as stale, so the window can notice it
    /// has become so between two passes.
    pub ttl: SignedDuration,
}

impl Snapshot {
    /// Whether it is stale at `now`: it was, or it has aged past its TTL since.
    pub fn is_stale(&self, now: Timestamp) -> bool {
        self.stale || now.duration_since(self.fetched_at) > self.ttl
    }
}

/// What one pass found.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// When the next pass is due.
    pub next_run: Timestamp,
    /// What changed, already filtered by what the user chose to hear about.
    pub changes: Vec<Change>,
    /// The weather to show, if there is any.
    pub snapshot: Option<Snapshot>,
}

/// One pass of the work, with the network as an input.
pub struct Engine {
    root: PathBuf,
    fetch: Box<dyn Fetch>,
    cancel: Cancel,
    detector: Option<Detector>,
    failures: u32,
    retry_at: Option<Timestamp>,
    /// Set for a developer's simulation: it needs no setting, no place and no
    /// cache, and polls at this interval.
    simulated: Option<Duration>,
}

impl Engine {
    pub fn new(root: &Path, fetch: Box<dyn Fetch>, cancel: Cancel) -> Self {
        Self {
            root: root.to_path_buf(),
            fetch,
            cancel,
            detector: None,
            failures: 0,
            retry_at: None,
            simulated: None,
        }
    }

    /// An engine that is polled every `interval` regardless of the settings, for
    /// watching a change happen without waiting for the weather.
    pub fn simulating(
        root: &Path,
        fetch: Box<dyn Fetch>,
        cancel: Cancel,
        interval: Duration,
    ) -> Self {
        Self {
            simulated: Some(interval),
            ..Self::new(root, fetch, cancel)
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

    /// The thresholds the settings ask for.
    fn thresholds(settings: &ot_core::WeatherSettings) -> Vec<Threshold> {
        let mut thresholds = Vec::new();
        if settings.fall_enabled {
            thresholds.push(Threshold {
                direction: Direction::Fall,
                delta_c: f64::from(settings.fall_delta_c),
                window: SignedDuration::from_hours(i64::from(settings.fall_window_hours)),
            });
        }
        if settings.rise_enabled {
            thresholds.push(Threshold {
                direction: Direction::Rise,
                delta_c: f64::from(settings.rise_delta_c),
                window: SignedDuration::from_hours(i64::from(settings.rise_window_hours)),
            });
        }
        thresholds
    }

    /// The template of the condition a change is to, if it is to one.
    pub fn template_for(&self, change: &Change) -> Option<ot_store::reminders::Template> {
        let Change::Condition { to, .. } = change else {
            return None;
        };
        let name = to.name()?;
        Store::open(&self.root)
            .and_then(|store| store.template_for_weather(name))
            .ok()
            .flatten()
    }

    /// One pass. Never panics and never blocks for longer than a fetch's own
    /// timeout; whatever fails is logged and the next pass carries on.
    pub fn run(&mut self, now: Timestamp) -> Report {
        let mut settings = self.settings();
        settings.weather = settings.weather.clamped();
        let idle = |next: SignedDuration| Report {
            next_run: now + next,
            changes: Vec::new(),
            snapshot: None,
        };
        if !settings.weather.enabled && self.simulated.is_none() {
            return idle(IDLE);
        }
        let store = match Store::open(&self.root) {
            Ok(store) => store,
            Err(error) => {
                tracing::warn!(%error, "the weather service could not open the store");
                return idle(SignedDuration::from_mins(1));
            }
        };
        let place = store.location().ok().flatten();
        let coordinates = match (&place, self.simulated) {
            (Some(place), _) => (place.latitude, place.longitude),
            (None, Some(_)) => (0.0, 0.0),
            (None, None) => return idle(IDLE),
        };

        let interval = self.simulated.map_or_else(
            || SignedDuration::from_mins(i64::from(settings.weather.poll_minutes())),
            |every| SignedDuration::try_from(every).unwrap_or(SignedDuration::from_secs(5)),
        );
        let ttl = DEFAULT_TTL.max(interval * 2);
        let cached = if self.simulated.is_some() {
            None
        } else {
            store.weather().ok().flatten()
        };
        let waiting = self.retry_at.is_some_and(|at| now < at);
        let due = !waiting
            && cached
                .as_ref()
                .is_none_or(|cache| now >= cache.fetched_at + interval);

        let detector = self.detector.get_or_insert_with(|| {
            let notified = store
                .setting(NOTIFIED)
                .ok()
                .flatten()
                .and_then(|name| Condition::from_name(&name));
            Detector::starting_from(notified)
        });

        let mut changes = Vec::new();
        let mut snapshot = cached.as_ref().and_then(|cache| {
            let observation = parse(cache.payload_json.as_bytes()).ok()?;
            Some(Snapshot {
                condition: classify(observation.code),
                temperature_c: observation.temperature_c,
                fetched_at: cache.fetched_at,
                stale: cache.is_stale(now, ttl),
                ttl,
            })
        });
        let mut next_run = cached
            .as_ref()
            .map_or(now + interval, |cache| cache.fetched_at + interval);

        if due {
            match self.fetch.fetch(coordinates.0, coordinates.1, &self.cancel) {
                Ok((observation, body)) => {
                    if self.cancel.is_cancelled() {
                        return idle(IDLE);
                    }
                    if self.simulated.is_none()
                        && let Err(error) = store.write_weather(now, &body)
                    {
                        tracing::warn!(%error, "the weather could not be cached");
                    }
                    self.failures = 0;
                    self.retry_at = None;
                    snapshot = Some(Snapshot {
                        condition: classify(observation.code),
                        temperature_c: observation.temperature_c,
                        fetched_at: now,
                        stale: false,
                        ttl,
                    });
                    next_run = now + interval;
                    let found =
                        detector.observe(&observation, true, &Self::thresholds(&settings.weather));
                    changes = found
                        .into_iter()
                        .filter(|change| match change {
                            Change::Condition { to, .. } => to
                                .name()
                                .is_some_and(|name| settings.weather.notifies(name)),
                            Change::Temperature { .. } => true,
                        })
                        .collect();
                    if let Some(name) = detector.notified().and_then(Condition::name)
                        && let Err(error) = store.set_setting(NOTIFIED, name)
                    {
                        tracing::warn!(%error, "the last weather class could not be kept");
                    }
                    tracing::info!(
                        code = observation.code,
                        temperature = observation.temperature_c,
                        changes = changes.len(),
                        "the weather was fetched"
                    );
                }
                Err(error) => {
                    self.failures += 1;
                    let wait = fetch::next_after_failure(&error, self.failures)
                        .and_then(|wait| SignedDuration::try_from(wait).ok())
                        .unwrap_or(interval);
                    self.retry_at = Some(now + wait);
                    next_run = now + wait;
                    tracing::warn!(
                        %error,
                        failures = self.failures,
                        retry_in_secs = wait.as_secs(),
                        "the weather fetch failed; keeping the last answer"
                    );
                }
            }
        } else if waiting && let Some(at) = self.retry_at {
            // Nothing is due while a retry waits: look again when it does.
            next_run = at;
        }
        Report {
            // Never in the past or the very next instant: a pass that finds
            // nothing due must not be run again at once.
            next_run: next_run.max(now + SignedDuration::from_secs(1)),
            changes,
            snapshot,
        }
    }
}

/// What the window tells the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Msg {
    /// The settings or the saved place changed: look again now.
    Changed,
    Stop,
}

/// What the service tells the window.
#[derive(Debug, Clone, PartialEq)]
pub enum Note {
    /// The weather as it stands, for the pane (`docs/specs/weather.md` §7).
    Snapshot(Snapshot),
    /// A change, with the template of the condition it is to, if it is to one
    /// (the store is read here, on the service's thread, not the window's).
    Change(Change, Option<Box<ot_store::reminders::Template>>),
}

/// The thread, and the window's hold on it.
pub struct Service {
    control: Sender<Msg>,
    notes: Receiver<Note>,
    cancel: Cancel,
}

impl Service {
    /// Start the service on `root`'s store. `simulate` is a developer's weather
    /// code: the service then reads a pretend sky that is clear once and then that
    /// code, every few seconds, whatever the settings say.
    pub fn start(root: &Path, simulate: Option<u8>) -> Self {
        let (control, incoming) = channel();
        let (note_out, notes) = sync_channel(8);
        let cancel = Cancel::new();
        let engine = match simulate {
            Some(code) => Engine::simulating(
                root,
                Box::new(Simulation::new(code)),
                cancel.clone(),
                Duration::from_secs(5),
            ),
            None => Engine::new(root, Box::new(fetch::Client::new()), cancel.clone()),
        };
        if let Err(error) = std::thread::Builder::new()
            .name("weather-service".into())
            .spawn(move || serve(engine, &incoming, &note_out))
        {
            tracing::error!(%error, "the weather service could not be started");
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
            if let Some(snapshot) = report.snapshot {
                // The next pass sends the weather again, so a full channel only
                // delays it: weather updates coalesce, a change must not.
                let _ = notes.try_send(Note::Snapshot(snapshot));
            }
            for change in report.changes {
                let template = engine.template_for(&change).map(Box::new);
                // A change must not be lost to a full channel without a word.
                if notes.try_send(Note::Change(change, template)).is_err() {
                    tracing::error!("a weather change was dropped: the window is not keeping up");
                }
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

/// A pretend sky for a developer: clear on the first look, then `code`, each look
/// a minute after the last so the detector sees distinct readings. It is how the
/// exit criterion "a condition change triggers a toast in simulation" is watched
/// without waiting for rain.
pub struct Simulation {
    code: u8,
    looks: std::sync::atomic::AtomicU32,
}

impl Simulation {
    pub fn new(code: u8) -> Self {
        Self {
            code,
            looks: std::sync::atomic::AtomicU32::new(0),
        }
    }
}

impl Fetch for Simulation {
    fn fetch(&self, _: f64, _: f64, _: &Cancel) -> Result<(Observation, String), FetchError> {
        let look = self.looks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let observation = Observation {
            at: Timestamp::UNIX_EPOCH + SignedDuration::from_mins(i64::from(look) * 60 + 1),
            temperature_c: 28.0,
            code: if look == 0 { 0 } else { self.code },
            hourly: Vec::new(),
        };
        Ok((observation, String::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ot_store::location::{Origin, StoredLocation};
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};

    fn at(text: &str) -> Timestamp {
        text.parse().expect("an instant")
    }

    const T0: &str = "2026-11-26T10:00:00Z";

    fn minutes(from: &str, n: i64) -> Timestamp {
        at(from) + SignedDuration::from_mins(n)
    }

    /// An observation `n` minutes after T0, with this code, temperature and the
    /// hourly points `(minutes before, °C)`.
    fn observation(n: i64, code: u8, now_c: f64, history: &[(i64, f64)]) -> Observation {
        Observation {
            at: minutes(T0, n),
            temperature_c: now_c,
            code,
            hourly: history
                .iter()
                .map(|(before, degrees)| (minutes(T0, n - before), *degrees))
                .collect(),
        }
    }

    /// A fetch that answers from a script and counts its calls.
    #[derive(Clone, Default)]
    struct Script {
        answers: Arc<Mutex<VecDeque<Result<Observation, FetchError>>>>,
        calls: Arc<AtomicU32>,
        asked: Arc<Mutex<Vec<(f64, f64)>>>,
        cancel_inside: Arc<Mutex<Option<Cancel>>>,
    }

    impl Script {
        fn push(&self, answer: Result<Observation, FetchError>) {
            self.answers.lock().expect("lock").push_back(answer);
        }
        fn calls(&self) -> u32 {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl Fetch for Script {
        fn fetch(
            &self,
            latitude: f64,
            longitude: f64,
            _: &Cancel,
        ) -> Result<(Observation, String), FetchError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.asked.lock().expect("lock").push((latitude, longitude));
            if let Some(cancel) = self.cancel_inside.lock().expect("lock").as_ref() {
                cancel.cancel();
            }
            let observation = self
                .answers
                .lock()
                .expect("lock")
                .pop_front()
                .expect("the script ran out of answers")?;
            let body = format!(
                r#"{{"utc_offset_seconds":0,"current":{{"time":"{}","temperature_2m":{},"weather_code":{}}}}}"#,
                observation.at.strftime("%Y-%m-%dT%H:%M"),
                observation.temperature_c,
                observation.code
            );
            Ok((observation, body))
        }
    }

    struct Rig {
        dir: tempfile::TempDir,
        engine: Engine,
        script: Script,
        cancel: Cancel,
    }

    fn weather_on() -> Settings {
        let mut settings = Settings::default();
        settings.weather.enabled = true;
        settings
    }

    fn rig_with(settings: &Settings, place: bool) -> Rig {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let root = DataRoot::resolve(Some(dir.path())).expect("resolves");
        settings.save(&root).expect("saved");
        // As `main` does: the database exists before any thread uses it.
        let store = Store::open(dir.path()).expect("created");
        if place {
            store
                .save_manual_location(&StoredLocation {
                    name: "Kuala Lumpur".into(),
                    latitude: 3.15,
                    longitude: 101.69,
                    tz: "Asia/Kuala_Lumpur".into(),
                    country_code: Some("MY".into()),
                    origin: Origin::Manual,
                    updated_at: at(T0),
                })
                .expect("saved");
        }
        drop(store);
        let script = Script::default();
        let cancel = Cancel::new();
        let engine = Engine::new(dir.path(), Box::new(script.clone()), cancel.clone());
        Rig {
            dir,
            engine,
            script,
            cancel,
        }
    }

    fn rig() -> Rig {
        rig_with(&weather_on(), true)
    }

    fn store(rig: &Rig) -> Store {
        Store::open(rig.dir.path()).expect("opens")
    }

    #[test]
    fn with_weather_off_nothing_is_fetched_and_nothing_cached() {
        let mut rig = rig_with(&Settings::default(), true);
        let report = rig.engine.run(at(T0));
        assert_eq!(rig.script.calls(), 0, "off means no outbound request");
        assert_eq!(report.snapshot, None);
        assert_eq!(report.next_run, minutes(T0, 10));
        assert_eq!(store(&rig).weather().expect("reads"), None);
    }

    #[test]
    fn without_a_saved_place_nothing_is_fetched() {
        let mut rig = rig_with(&weather_on(), false);
        let report = rig.engine.run(at(T0));
        assert_eq!(rig.script.calls(), 0);
        assert_eq!(report.snapshot, None);
    }

    #[test]
    fn the_first_pass_fetches_for_the_saved_place_caches_it_and_is_only_a_baseline() {
        let mut rig = rig();
        rig.script.push(Ok(observation(0, 61, 24.0, &[])));
        let report = rig.engine.run(at(T0));
        assert_eq!(rig.script.calls(), 1);
        assert_eq!(*rig.script.asked.lock().expect("lock"), [(3.15, 101.69)]);
        assert!(report.changes.is_empty(), "the first reading says nothing");
        assert_eq!(
            report.snapshot,
            Some(Snapshot {
                condition: Condition::Rain,
                temperature_c: 24.0,
                fetched_at: at(T0),
                stale: false,
                ttl: SignedDuration::from_hours(2)
            })
        );
        assert_eq!(report.next_run, minutes(T0, 30));
        let cached = store(&rig).weather().expect("reads").expect("cached");
        assert_eq!(cached.fetched_at, at(T0));
        assert!(cached.payload_json.contains("\"weather_code\":61"));
    }

    #[test]
    fn nothing_is_fetched_again_before_the_interval_and_the_cache_answers() {
        let mut rig = rig();
        rig.script.push(Ok(observation(0, 0, 28.0, &[])));
        rig.engine.run(at(T0));
        let report = rig.engine.run(minutes(T0, 10));
        assert_eq!(
            rig.script.calls(),
            1,
            "no polling loop tighter than the interval"
        );
        let snapshot = report.snapshot.expect("from the cache");
        assert_eq!(
            (snapshot.condition, snapshot.stale),
            (Condition::Clear, false)
        );
        assert_eq!(report.next_run, minutes(T0, 30));
        // At the interval it is due.
        rig.script.push(Ok(observation(30, 0, 28.0, &[])));
        rig.engine.run(minutes(T0, 30));
        assert_eq!(rig.script.calls(), 2);
    }

    #[test]
    fn a_change_is_told_once_on_its_second_sighting() {
        let mut rig = rig();
        for (n, code) in [(0, 0), (30, 61), (60, 61), (90, 61)] {
            rig.script.push(Ok(observation(n, code, 25.0, &[])));
        }
        let found: Vec<Vec<Change>> = [0, 30, 60, 90]
            .iter()
            .map(|n| rig.engine.run(minutes(T0, *n)).changes)
            .collect();
        assert!(found[0].is_empty() && found[1].is_empty() && found[3].is_empty());
        assert_eq!(
            found[2],
            [Change::Condition {
                from: Condition::Clear,
                to: Condition::Rain
            }]
        );
    }

    #[test]
    fn a_class_the_user_did_not_choose_is_not_told() {
        let mut settings = weather_on();
        settings.weather.notify_classes = vec!["rain".into()];
        let mut rig = rig_with(&settings, true);
        for (n, code) in [(0, 0), (30, 95), (60, 95)] {
            rig.script.push(Ok(observation(n, code, 25.0, &[])));
        }
        let told: usize = [0, 30, 60]
            .iter()
            .map(|n| rig.engine.run(minutes(T0, *n)).changes.len())
            .sum();
        assert_eq!(told, 0, "a storm was not asked for");
    }

    #[test]
    fn a_temperature_fall_is_told_through_the_settings_thresholds() {
        let mut rig = rig();
        // 31 two hours ago, 25 now: 6 down, over the default 5 in 3 hours.
        rig.script.push(Ok(observation(0, 0, 25.0, &[(120, 31.0)])));
        let changes = rig.engine.run(at(T0)).changes;
        assert!(matches!(
            changes.as_slice(),
            [Change::Temperature {
                direction: Direction::Fall,
                ..
            }]
        ));
        // And with the fall switched off, it is not.
        let mut settings = weather_on();
        settings.weather.fall_enabled = false;
        let mut quiet = rig_with(&settings, true);
        quiet
            .script
            .push(Ok(observation(0, 0, 25.0, &[(120, 31.0)])));
        assert!(quiet.engine.run(at(T0)).changes.is_empty());
    }

    #[test]
    fn a_failed_fetch_keeps_the_cache_backs_off_and_tells_nothing() {
        let mut rig = rig();
        rig.script.push(Ok(observation(0, 0, 28.0, &[])));
        rig.engine.run(at(T0));
        // The next poll fails, three times, then the interval's worth.
        for _ in 0..3 {
            rig.script.push(Err(FetchError::Network("down".into())));
        }
        let first = rig.engine.run(minutes(T0, 30));
        assert_eq!(first.next_run, minutes(T0, 31), "one minute");
        assert!(first.changes.is_empty());
        let snapshot = first.snapshot.expect("the cache is kept");
        assert_eq!(
            (snapshot.condition, snapshot.fetched_at),
            (Condition::Clear, at(T0))
        );
        // While it waits nothing is asked.
        let calls = rig.script.calls();
        rig.engine
            .run(minutes(T0, 30) + SignedDuration::from_secs(30));
        assert_eq!(rig.script.calls(), calls, "the retry has not come");
        // Then five, then fifteen minutes.
        let second = rig.engine.run(minutes(T0, 31));
        assert_eq!(second.next_run, minutes(T0, 36));
        let third = rig.engine.run(minutes(T0, 36));
        assert_eq!(third.next_run, minutes(T0, 51));
    }

    #[test]
    fn offline_the_cache_is_shown_and_marked_stale_only_when_old() {
        let mut rig = rig();
        rig.script.push(Ok(observation(0, 61, 24.0, &[])));
        rig.engine.run(at(T0));
        let mut stale_at = |n: i64| {
            rig.script.push(Err(FetchError::Network("down".into())));
            rig.engine
                .run(minutes(T0, n))
                .snapshot
                .expect("cache")
                .stale
        };
        // Two polls' worth (an hour) is not yet old; past two hours is.
        assert!(!stale_at(60));
        assert!(!stale_at(120));
        assert!(stale_at(121 + 20));
    }

    #[test]
    fn stale_data_never_feeds_the_detector() {
        let mut rig = rig();
        rig.script.push(Ok(observation(0, 0, 28.0, &[])));
        rig.engine.run(at(T0));
        // Hours of failure, then an answer that is rain: only one sighting.
        rig.script.push(Err(FetchError::Network("down".into())));
        rig.engine.run(minutes(T0, 200));
        rig.script.push(Ok(observation(260, 61, 28.0, &[])));
        let report = rig.engine.run(minutes(T0, 260));
        assert!(report.changes.is_empty(), "one sighting is not a change");
    }

    #[test]
    fn what_was_told_survives_a_restart() {
        let mut rig = rig();
        rig.script.push(Ok(observation(0, 0, 28.0, &[])));
        rig.engine.run(at(T0));
        // A new engine on the same data folder: not a first reading again.
        let again = Script::default();
        let mut engine = Engine::new(rig.dir.path(), Box::new(again.clone()), Cancel::new());
        again.push(Ok(observation(40, 61, 28.0, &[])));
        again.push(Ok(observation(80, 61, 28.0, &[])));
        assert!(engine.run(minutes(T0, 40)).changes.is_empty(), "rain once");
        let told = engine.run(minutes(T0, 80)).changes;
        assert_eq!(
            told,
            [Change::Condition {
                from: Condition::Clear,
                to: Condition::Rain
            }],
            "told from the clear it remembered, not from a fresh baseline"
        );
    }

    #[test]
    fn a_cancel_during_the_fetch_tells_nothing_and_caches_nothing() {
        let mut rig = rig();
        *rig.script.cancel_inside.lock().expect("lock") = Some(rig.cancel.clone());
        rig.script.push(Ok(observation(0, 61, 28.0, &[])));
        let report = rig.engine.run(at(T0));
        assert!(report.changes.is_empty() && report.snapshot.is_none());
        assert_eq!(store(&rig).weather().expect("reads"), None);
    }

    #[test]
    fn a_snapshot_becomes_stale_when_it_ages_past_its_ttl_between_two_passes() {
        let snapshot = Snapshot {
            condition: Condition::Rain,
            temperature_c: 24.0,
            fetched_at: at(T0),
            stale: false,
            ttl: SignedDuration::from_hours(2),
        };
        assert!(
            !snapshot.is_stale(minutes(T0, 120)),
            "exactly the TTL is fresh"
        );
        assert!(snapshot.is_stale(minutes(T0, 121)));
        assert!(
            !snapshot.is_stale(minutes(T0, -30)),
            "a clock that went back is not old"
        );
        let already = Snapshot {
            stale: true,
            ..snapshot
        };
        assert!(already.is_stale(at(T0)), "stale when made stays stale");
    }

    #[test]
    fn the_template_of_the_condition_a_change_is_to_is_attached_and_a_temperature_move_has_none() {
        let rig = rig();
        store(&rig)
            .set_weather_template(
                "storm",
                &ot_store::reminders::PrayerTemplateEdit {
                    title: Some("Lightning!".into()),
                    body: None,
                    accent: None,
                    image: None,
                    authored_lang: "en-US",
                    sound_mode: Some("chime".into()),
                    sound: None,
                },
            )
            .expect("saved");
        let storm = Change::Condition {
            from: Condition::Clear,
            to: Condition::Storm,
        };
        let template = rig.engine.template_for(&storm).expect("storm has one");
        assert_eq!(template.custom_title.as_deref(), Some("Lightning!"));
        assert_eq!(template.sound_mode.as_deref(), Some("chime"));
        let fog = Change::Condition {
            from: Condition::Clear,
            to: Condition::Fog,
        };
        assert_eq!(
            rig.engine
                .template_for(&fog)
                .expect("fog has one")
                .custom_title,
            None
        );
        let fall = Change::Temperature {
            direction: Direction::Fall,
            delta_c: 6.0,
            from_c: 31.0,
            to_c: 25.0,
            window: SignedDuration::from_hours(3),
        };
        assert_eq!(rig.engine.template_for(&fall), None);
    }

    #[test]
    fn a_simulation_needs_no_setting_place_or_cache_and_a_change_appears_on_the_third_look() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        DataRoot::resolve(Some(dir.path())).expect("resolves");
        drop(Store::open(dir.path()).expect("created"));
        let mut engine = Engine::simulating(
            dir.path(),
            Box::new(Simulation::new(95)),
            Cancel::new(),
            Duration::from_secs(5),
        );
        let mut found = Vec::new();
        for look in 0..4 {
            let now = at(T0) + SignedDuration::from_secs(5 * look);
            found.push(engine.run(now).changes);
        }
        assert!(found[0].is_empty() && found[1].is_empty() && found[3].is_empty());
        assert_eq!(
            found[2],
            [Change::Condition {
                from: Condition::Clear,
                to: Condition::Storm
            }]
        );
        assert_eq!(
            Store::open(dir.path())
                .expect("opens")
                .weather()
                .expect("reads"),
            None,
            "a simulation caches nothing"
        );
    }
}
