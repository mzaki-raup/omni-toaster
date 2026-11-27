//! The update service (FEAT-0080, `docs/specs/updater.md` §4, §7 and §8).
//!
//! A thread that does the network work the Updates pane asks for — look for a newer
//! release, then verify, download and stage it — so the window never waits on the
//! network. It also looks once a day on its own when the user has asked it to.
//!
//! - **Off means no requests.** The settings are read from disk before every
//!   request, so turning the check off stops the next one even if a button was
//!   pressed a moment before.
//! - **Cancel stops the work.** The flag in flight is shared, so a cancel reaches a
//!   download between two chunks; the window dropping the service cancels too.
//! - **A day at most.** The time of the last request is kept in
//!   `cache/update/last-check`, and an automatic check waits until it is a day old.
//!   A failed check counts: there is no retry loop.
//! - **It never installs by itself.** It stages; swapping the program in is the
//!   user's Restart button.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender, channel, sync_channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use jiff::Timestamp;
use ot_core::{DataRoot, Settings};
use ot_update::check::{CheckError, Client, Outcome, PrepareError};
use ot_update::download::{DownloadError, Downloader};
use ot_update::release::{self, Release};
use ot_update::stage::{self, StageError};
use ot_update::swap::Layout;
use ot_update::{Cancel, verify};
use semver::Version;

/// How often the thread looks at the clock when nothing was asked of it, so a
/// laptop that slept wakes to the right time rather than to a long wait.
const WAKE: Duration = Duration::from_secs(60);

/// An automatic check is due when the last request is older than this.
const DAY_SECS: i64 = 24 * 60 * 60;

/// What went wrong, in the terms the pane has words for. Nothing here carries
/// text from the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The forge could not be reached or answered with an error.
    Network,
    /// No release has been published at all.
    NoRelease,
    /// The signature, the manifest or the release did not check out. Nothing was
    /// downloaded or installed.
    Rejected,
    /// The download is not what was signed, and was deleted.
    BadDownload,
    /// The folder the program is in cannot be written to.
    FolderNotWritable,
    /// This copy is older than the release will update in place.
    TooOld(Version),
    /// The last update did not start properly and was undone.
    RolledBack,
    /// Anything else; the log has the detail.
    Other,
}

/// Why an install did not stage anything.
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error(transparent)]
    Prepare(#[from] PrepareError),
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error(transparent)]
    Stage(#[from] StageError),
}

impl InstallError {
    fn cancelled(&self) -> bool {
        matches!(
            self,
            Self::Prepare(PrepareError::Cancelled) | Self::Download(DownloadError::Cancelled)
        )
    }

    /// What the pane says. Every failure is named; none is a silent fallback.
    pub fn problem(&self) -> Problem {
        match self {
            Self::Prepare(PrepareError::Network(_) | PrepareError::Http(_)) => Problem::Network,
            Self::Prepare(PrepareError::TooLarge) => Problem::Network,
            Self::Prepare(PrepareError::ManualUpdateNeeded(version)) => {
                Problem::TooOld(version.clone())
            }
            Self::Prepare(
                PrepareError::Verify(_)
                | PrepareError::VersionMismatch { .. }
                | PrepareError::NotNewer(_)
                | PrepareError::MissingAsset(_),
            ) => Problem::Rejected,
            Self::Prepare(PrepareError::Cancelled) => Problem::Other,
            Self::Download(DownloadError::Network(_) | DownloadError::Http(_)) => Problem::Network,
            Self::Download(
                DownloadError::TooLarge(_)
                | DownloadError::WrongSize { .. }
                | DownloadError::WrongDigest,
            ) => Problem::BadDownload,
            Self::Download(DownloadError::Cache { .. } | DownloadError::Cancelled) => {
                Problem::Other
            }
            Self::Stage(StageError::NotWritable { .. }) => Problem::FolderNotWritable,
            Self::Stage(StageError::CopyMismatch) => Problem::BadDownload,
            Self::Stage(StageError::BadPath(_)) => Problem::Other,
        }
    }
}

/// What a failed check is called in the pane.
pub fn check_problem(error: &CheckError) -> Problem {
    match error {
        CheckError::Network(_) | CheckError::Http(_) | CheckError::TooLarge => Problem::Network,
        CheckError::NoRelease => Problem::NoRelease,
        CheckError::Release(_) | CheckError::Cancelled => Problem::Other,
    }
}

/// The network, as an input, so the policy can be tested without one.
pub trait Backend: Send + Sync + 'static {
    fn check(&self, running: &Version, cancel: &Cancel) -> Result<Outcome, CheckError>;

    /// Verify, download and stage `release` beside `exe`, with the download kept in
    /// `cache`. Returns the version staged.
    fn install(
        &self,
        release: &Release,
        running: &Version,
        exe: &Path,
        cache: &Path,
        cancel: &Cancel,
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<Version, InstallError>;
}

/// The real thing: this copy's own forge.
pub struct Forge {
    client: Client,
    downloader: Downloader,
}

impl Forge {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            downloader: Downloader::new(),
        }
    }
}

impl Default for Forge {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for Forge {
    fn check(&self, running: &Version, cancel: &Cancel) -> Result<Outcome, CheckError> {
        self.client.check(running, cancel)
    }

    fn install(
        &self,
        release: &Release,
        running: &Version,
        exe: &Path,
        cache: &Path,
        cancel: &Cancel,
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<Version, InstallError> {
        let manifest =
            self.client
                .verified_manifest(release, running, verify::PUBLIC_KEY, cancel)?;
        let downloaded =
            self.downloader
                .download(&manifest.exe, &manifest.version, cache, cancel, progress)?;
        let staged = stage::stage(&downloaded, exe, &manifest.exe.sha256);
        // The staged copy is the one that is used; the download has done its job.
        let _ = std::fs::remove_file(&downloaded);
        staged?;
        Ok(manifest.version)
    }
}

/// What the thread tells the window.
#[derive(Debug, Clone, PartialEq)]
pub enum Note {
    /// A request is on its way.
    Checking,
    Checked {
        outcome: Outcome,
        at: Timestamp,
    },
    Progress {
        version: Version,
        done: u64,
        total: u64,
    },
    /// The new program is staged beside this one, waiting for a restart.
    Staged(Version),
    Failed(Problem),
    /// The work was cancelled, or refused because checking is off: back to idle.
    Stopped,
}

enum Command {
    Check,
    Install(Box<Release>),
    Stop,
}

/// The window's end of the service.
pub struct Service {
    commands: Sender<Command>,
    notes: Receiver<Note>,
    current: Arc<Mutex<Option<Cancel>>>,
}

impl Service {
    /// Start the thread for the real forge.
    pub fn start(root: &Path) -> Self {
        Self::start_with(root, Box::new(Forge::new()), WAKE)
    }

    /// Start it with another backend and another wake interval: the tests.
    pub fn start_with(root: &Path, backend: Box<dyn Backend>, wake: Duration) -> Self {
        let (commands, command_rx) = channel();
        let (note_tx, notes) = sync_channel(32);
        let current = Arc::new(Mutex::new(None));
        let worker = Worker {
            root: root.to_path_buf(),
            backend,
            notes: note_tx,
            current: Arc::clone(&current),
        };
        if let Err(error) = std::thread::Builder::new()
            .name("update-service".into())
            .spawn(move || worker.run(&command_rx, wake))
        {
            tracing::warn!(%error, "the update service could not start");
        }
        Self {
            commands,
            notes,
            current,
        }
    }

    /// Ask for a check now. The answer arrives as a [`Note`].
    pub fn check(&self) {
        let _ = self.commands.send(Command::Check);
    }

    /// Verify, download and stage `release`.
    pub fn install(&self, release: Release) {
        let _ = self.commands.send(Command::Install(Box::new(release)));
    }

    /// Stop what is in flight: the transfer ends at its next chunk.
    pub fn cancel(&self) {
        if let Ok(current) = self.current.lock()
            && let Some(cancel) = current.as_ref()
        {
            cancel.cancel();
        }
    }

    /// The next thing the thread has to say, if anything. Drained each frame.
    pub fn poll(&self) -> Option<Note> {
        self.notes.try_recv().ok()
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        self.cancel();
        let _ = self.commands.send(Command::Stop);
    }
}

struct Worker {
    root: PathBuf,
    backend: Box<dyn Backend>,
    notes: SyncSender<Note>,
    current: Arc<Mutex<Option<Cancel>>>,
}

impl Worker {
    fn run(&self, commands: &Receiver<Command>, wake: Duration) {
        loop {
            let alive = match commands.recv_timeout(wake) {
                Ok(Command::Check) => self.check(true),
                Ok(Command::Install(release)) => self.install(&release),
                Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => {
                    if self.automatic_check_due() {
                        self.check(false)
                    } else {
                        true
                    }
                }
            };
            if !alive {
                return;
            }
        }
    }

    fn settings(&self) -> Settings {
        DataRoot::resolve(Some(&self.root))
            .map_err(|error| error.to_string())
            .and_then(|root| Settings::load(&root).map_err(|error| error.to_string()))
            .unwrap_or_else(|reason| {
                tracing::warn!(%reason, "the settings could not be read; using the defaults");
                Settings::default()
            })
    }

    fn last_check_file(&self) -> PathBuf {
        Layout::new(Path::new("OmniToaster.exe"), &self.root)
            .cache
            .join("last-check")
    }

    fn automatic_check_due(&self) -> bool {
        let settings = self.settings().updates;
        if !(settings.enabled && settings.automatic) {
            return false;
        }
        let last = std::fs::read_to_string(self.last_check_file())
            .ok()
            .and_then(|text| text.trim().parse::<i64>().ok());
        match last {
            Some(seconds) => Timestamp::now().as_second() - seconds >= DAY_SECS,
            None => true,
        }
    }

    fn record_check(&self) {
        let path = self.last_check_file();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(error) = std::fs::write(&path, Timestamp::now().as_second().to_string()) {
            tracing::warn!(%error, "the time of the update check could not be kept");
        }
    }

    /// Tell the window; `false` when it is gone and the thread should end.
    fn tell(&self, note: Note) -> bool {
        self.notes.send(note).is_ok()
    }

    fn begin(&self) -> Cancel {
        let cancel = Cancel::new();
        if let Ok(mut current) = self.current.lock() {
            *current = Some(cancel.clone());
        }
        cancel
    }

    fn end(&self) {
        if let Ok(mut current) = self.current.lock() {
            *current = None;
        }
    }

    fn check(&self, manual: bool) -> bool {
        if !self.settings().updates.enabled {
            // Nothing is sent, and a button pressed a moment before is let go.
            tracing::info!("an update check was asked for while checking is off; nothing sent");
            return self.tell(Note::Stopped);
        }
        if !self.tell(Note::Checking) {
            return false;
        }
        let cancel = self.begin();
        self.record_check();
        let result = self.backend.check(&release::running(), &cancel);
        self.end();
        let note = match result {
            Ok(outcome) => Note::Checked {
                outcome,
                at: Timestamp::now(),
            },
            Err(CheckError::Cancelled) => Note::Stopped,
            Err(error) => {
                tracing::warn!(%error, manual, "the update check failed");
                Note::Failed(check_problem(&error))
            }
        };
        self.tell(note)
    }

    fn install(&self, release: &Release) -> bool {
        if !self.settings().updates.enabled {
            return self.tell(Note::Stopped);
        }
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(error) => {
                tracing::warn!(%error, "the running program's path is unknown");
                return self.tell(Note::Failed(Problem::Other));
            }
        };
        let cache = Layout::new(&exe, &self.root).cache;
        let cancel = self.begin();
        let notes = self.notes.clone();
        let version = release.version.clone();
        let mut progress = move |done, total| {
            // Progress is dropped when the window is behind; the result is not.
            let _ = notes.try_send(Note::Progress {
                version: version.clone(),
                done,
                total,
            });
        };
        let result = self.backend.install(
            release,
            &release::running(),
            &exe,
            &cache,
            &cancel,
            &mut progress,
        );
        self.end();
        let note = match result {
            Ok(version) => Note::Staged(version),
            Err(error) if error.cancelled() => Note::Stopped,
            Err(error) => {
                tracing::warn!(%error, "the update was not installed");
                Note::Failed(error.problem())
            }
        };
        self.tell(note)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Counts what was asked of it and answers from a script.
    struct Fake {
        checks: Arc<AtomicUsize>,
        installs: Arc<AtomicUsize>,
        outcome: Result<Outcome, CheckError>,
        /// When set, an install blocks until it is cancelled, then reports so.
        install_waits: bool,
    }

    fn release(version: &str) -> Release {
        Release {
            version: Version::parse(version).expect("version"),
            tag: format!("v{version}"),
            page_url: "https://example.test/r".into(),
            assets: Vec::new(),
        }
    }

    impl Backend for Fake {
        fn check(&self, _running: &Version, _cancel: &Cancel) -> Result<Outcome, CheckError> {
            self.checks.fetch_add(1, Ordering::SeqCst);
            self.outcome.clone()
        }

        fn install(
            &self,
            release: &Release,
            _running: &Version,
            _exe: &Path,
            _cache: &Path,
            cancel: &Cancel,
            progress: &mut dyn FnMut(u64, u64),
        ) -> Result<Version, InstallError> {
            self.installs.fetch_add(1, Ordering::SeqCst);
            progress(10, 100);
            if self.install_waits {
                while !cancel.is_cancelled() {
                    std::thread::sleep(Duration::from_millis(10));
                }
                return Err(InstallError::Download(DownloadError::Cancelled));
            }
            Ok(release.version.clone())
        }
    }

    struct Rig {
        service: Service,
        checks: Arc<AtomicUsize>,
        installs: Arc<AtomicUsize>,
        _dir: tempfile::TempDir,
        root: PathBuf,
    }

    fn rig(settings: impl FnOnce(&mut Settings), install_waits: bool, wake_ms: u64) -> Rig {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().to_path_buf();
        let data = DataRoot::resolve(Some(&root)).expect("root");
        let mut all = Settings::default();
        settings(&mut all);
        all.save(&data).expect("saves");
        let checks = Arc::new(AtomicUsize::new(0));
        let installs = Arc::new(AtomicUsize::new(0));
        let fake = Fake {
            checks: Arc::clone(&checks),
            installs: Arc::clone(&installs),
            outcome: Ok(Outcome::Available(release("9.9.9"))),
            install_waits,
        };
        let service = Service::start_with(&root, Box::new(fake), Duration::from_millis(wake_ms));
        Rig {
            service,
            checks,
            installs,
            _dir: dir,
            root,
        }
    }

    /// The next note, or a failure rather than a hang.
    fn next(service: &Service) -> Note {
        let started = std::time::Instant::now();
        loop {
            if let Some(note) = service.poll() {
                return note;
            }
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "no note arrived"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn check_now_says_it_is_checking_then_what_it_found_and_keeps_the_time() {
        let rig = rig(|_| {}, false, 60_000);
        rig.service.check();
        assert_eq!(next(&rig.service), Note::Checking);
        match next(&rig.service) {
            Note::Checked {
                outcome: Outcome::Available(found),
                ..
            } => assert_eq!(found.version, Version::new(9, 9, 9)),
            other => panic!("{other:?}"),
        }
        assert_eq!(rig.checks.load(Ordering::SeqCst), 1);
        let kept =
            std::fs::read_to_string(rig.root.join("cache").join("update").join("last-check"))
                .expect("the time of the check is kept");
        assert!(kept.trim().parse::<i64>().is_ok(), "{kept}");
    }

    #[test]
    fn with_checking_off_nothing_is_sent_even_when_asked() {
        let rig = rig(|s| s.updates.enabled = false, false, 60_000);
        rig.service.check();
        assert_eq!(next(&rig.service), Note::Stopped);
        rig.service.install(release("9.9.9"));
        assert_eq!(next(&rig.service), Note::Stopped);
        assert_eq!(rig.checks.load(Ordering::SeqCst), 0, "no check was made");
        assert_eq!(
            rig.installs.load(Ordering::SeqCst),
            0,
            "no install was made"
        );
    }

    #[test]
    fn an_automatic_check_happens_when_chosen_and_never_when_not() {
        let on = rig(|s| s.updates.automatic = true, false, 30);
        assert_eq!(next(&on.service), Note::Checking);
        assert!(matches!(next(&on.service), Note::Checked { .. }));

        let off = rig(|_| {}, false, 30);
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(
            off.checks.load(Ordering::SeqCst),
            0,
            "automatic is off by default"
        );

        let disabled = rig(
            |s| {
                s.updates.automatic = true;
                s.updates.enabled = false;
            },
            false,
            30,
        );
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(
            disabled.checks.load(Ordering::SeqCst),
            0,
            "off beats automatic"
        );
    }

    #[test]
    fn an_automatic_check_waits_a_day_after_the_last_request() {
        let rig = rig(|s| s.updates.automatic = true, false, 30);
        assert_eq!(next(&rig.service), Note::Checking);
        assert!(matches!(next(&rig.service), Note::Checked { .. }));
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(rig.checks.load(Ordering::SeqCst), 1, "once, not every wake");
    }

    #[test]
    fn an_install_reports_progress_and_then_what_was_staged() {
        let rig = rig(|_| {}, false, 60_000);
        rig.service.install(release("9.9.9"));
        assert!(matches!(
            next(&rig.service),
            Note::Progress {
                done: 10,
                total: 100,
                ..
            }
        ));
        assert_eq!(next(&rig.service), Note::Staged(Version::new(9, 9, 9)));
    }

    #[test]
    fn cancel_stops_a_download_in_flight_and_the_service_goes_back_to_idle() {
        let rig = rig(|_| {}, true, 60_000);
        rig.service.install(release("9.9.9"));
        assert!(matches!(next(&rig.service), Note::Progress { .. }));
        rig.service.cancel();
        assert_eq!(next(&rig.service), Note::Stopped);
    }

    #[test]
    fn dropping_the_service_cancels_the_work_in_flight() {
        let rig = rig(|_| {}, true, 60_000);
        let Rig {
            service, installs, ..
        } = rig;
        service.install(release("9.9.9"));
        assert!(matches!(next(&service), Note::Progress { .. }));
        drop(service);
        // The fake only returns once cancelled; the thread then ends.
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(installs.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn every_failure_has_a_problem_the_pane_can_name() {
        assert_eq!(
            check_problem(&CheckError::Network("x".into())),
            Problem::Network
        );
        assert_eq!(check_problem(&CheckError::Http(500)), Problem::Network);
        assert_eq!(check_problem(&CheckError::NoRelease), Problem::NoRelease);
        let wrong = InstallError::Download(DownloadError::WrongDigest);
        assert_eq!(wrong.problem(), Problem::BadDownload);
        let old = InstallError::Prepare(PrepareError::ManualUpdateNeeded(Version::new(1, 0, 0)));
        assert_eq!(old.problem(), Problem::TooOld(Version::new(1, 0, 0)));
        let folder = InstallError::Stage(StageError::NotWritable {
            dir: PathBuf::from("C:/x"),
            source: std::io::Error::other("read-only"),
        });
        assert_eq!(folder.problem(), Problem::FolderNotWritable);
        let replay = InstallError::Prepare(PrepareError::NotNewer(Version::new(0, 1, 0)));
        assert_eq!(replay.problem(), Problem::Rejected);
    }
}
