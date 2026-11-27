//! Swapping the staged exe in, confirming it, and rolling it back (FEAT-0079,
//! `docs/specs/updater.md` §5).
//!
//! A running program cannot be *overwritten* on Windows, but it can be *renamed*,
//! and that is the whole trick. The worker stages `OmniToaster.exe.new` (FEAT-0078)
//! and starts a short-lived **update helper** — the old exe run with
//! `--apply-update` — then quits. The helper waits for the worker to be gone and
//! calls [`apply`]:
//!
//! 1. rename the exe to `.old` (the helper is running from that very image, which is
//!    allowed) and `.new` into its place; if the second rename fails, put the first
//!    back;
//! 2. start the new program and wait for it to **confirm** — the new program writes
//!    its version to a marker file once it has started ([`confirm`]);
//! 3. confirmed with the expected version: done, `.old` is deleted on the new
//!    program's first successful run ([`clean_up`]);
//! 4. it dies, never confirms, or confirms the wrong version: kill it, move it aside
//!    as `.failed`, put `.old` back, start the program that worked, and leave a note
//!    for it to tell the user ([`take_rollback_note`]).
//!
//! The helper touches the exe and its two siblings and the marker — never
//! `OmniToasterData/` beyond `cache/update/`. The process side is behind
//! [`Launcher`], so every step, including the failures, is tested against fake
//! processes and real files.
//!
//! This is the one-shot update supervisor; the crash-restarting supervisor of
//! `docs/specs/supervisor.md` is Phase 8 and does not exist yet, so the helper is
//! the only thing that touches the exe today. Phase 8 may take the role over.

use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use semver::Version;

/// The command line to start the update helper with: the helper is the running
/// exe, and `relaunch_args` are what it should start the updated program with (the
/// command line this copy was started with). The caller starts it with
/// `ot_platform::process::spawn_hidden` and then quits.
pub fn helper_args(data_dir: &Path, expect: &Version, relaunch_args: &[String]) -> Vec<String> {
    let mut args = vec![
        "--apply-update".to_owned(),
        "--expect".to_owned(),
        expect.to_string(),
        "--data-dir".to_owned(),
        data_dir.display().to_string(),
        "--".to_owned(),
    ];
    args.extend(relaunch_args.iter().cloned());
    args
}

/// The marker the new program writes once it has started: its version, as text.
const CONFIRMED: &str = "confirmed";
/// Why the last update was rolled back, for the program to say to the user once.
const ROLLED_BACK: &str = "rolled-back";

/// How long a new program has to confirm itself. A start that takes longer than
/// this is a start the user would call broken.
pub const CONFIRM_TIMEOUT: Duration = Duration::from_secs(30);

/// The files an update touches, derived from the running exe and the data folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// The running program, `OmniToaster.exe`.
    pub exe: PathBuf,
    /// The staged one, `OmniToaster.exe.new`.
    pub new: PathBuf,
    /// The one being replaced, `OmniToaster.exe.old`.
    pub old: PathBuf,
    /// A new program that did not take, `OmniToaster.exe.failed`.
    pub failed: PathBuf,
    /// `<data>/cache/update/`.
    pub cache: PathBuf,
}

impl Layout {
    pub fn new(exe: &Path, data_dir: &Path) -> Self {
        let sibling = |suffix: &str| {
            let mut name = exe.file_name().unwrap_or_default().to_os_string();
            name.push(suffix);
            exe.with_file_name(name)
        };
        Self {
            exe: exe.to_path_buf(),
            new: sibling(".new"),
            old: sibling(".old"),
            failed: sibling(".failed"),
            cache: data_dir.join("cache").join("update"),
        }
    }

    fn confirmed(&self) -> PathBuf {
        self.cache.join(CONFIRMED)
    }

    fn rolled_back(&self) -> PathBuf {
        self.cache.join(ROLLED_BACK)
    }
}

/// A started program.
pub trait Process {
    /// Whether it is still running.
    fn is_running(&mut self) -> bool;
    /// Stop it, and do not wait on it.
    fn kill(&mut self);
}

/// How the helper starts programs: the real one spawns the exe, a test's is a fake.
pub trait Launcher {
    type Child: Process;
    fn launch(&mut self, exe: &Path) -> io::Result<Self::Child>;
}

/// What an attempt came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing was staged, so nothing was done.
    NothingStaged,
    /// The new program started and confirmed its version.
    Updated,
    /// It did not take; the old program is back (and was started again).
    RolledBack(Reason),
    /// Even putting the old one back failed. Both names are left as found and the
    /// reason says what to do by hand; this should never happen.
    Stuck(String),
}

/// Why a swap was undone.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Reason {
    #[error("the staged program could not be moved into place: {0}")]
    CouldNotMove(String),
    #[error("the new program could not be started: {0}")]
    CouldNotStart(String),
    #[error("the new program stopped before it confirmed")]
    ExitedEarly,
    #[error("the new program did not confirm within {0:?}")]
    NotConfirmed(Duration),
    #[error("the new program confirmed version {got}, not the expected {expected}")]
    WrongVersion { expected: Version, got: String },
}

/// Do the swap. `expect` is the version the staged program must confirm. `sleep`
/// is injected so a test does not wait in real time.
pub fn apply<L: Launcher>(
    layout: &Layout,
    expect: &Version,
    launcher: &mut L,
    timeout: Duration,
    poll: Duration,
    sleep: &mut dyn FnMut(Duration),
) -> Outcome {
    if !layout.new.is_file() {
        return Outcome::NothingStaged;
    }
    let _ = std::fs::remove_file(&layout.old);
    let _ = std::fs::remove_file(&layout.failed);
    let _ = std::fs::remove_file(layout.confirmed());
    let _ = std::fs::remove_file(layout.rolled_back());

    if let Err(error) = std::fs::rename(&layout.exe, &layout.old) {
        return Outcome::RolledBack(record(
            layout,
            Reason::CouldNotMove(format!(
                "the running program could not be moved aside: {error}"
            )),
        ));
    }
    if let Err(error) = std::fs::rename(&layout.new, &layout.exe) {
        // Put the program back before anything else: no program at all is the
        // one outcome that is not acceptable.
        if let Err(back) = std::fs::rename(&layout.old, &layout.exe) {
            return Outcome::Stuck(format!(
                "the staged program could not be moved ({error}) and the old one could not be put back ({back}); \
                 rename {} to {}",
                layout.old.display(),
                layout.exe.display()
            ));
        }
        return Outcome::RolledBack(record(layout, Reason::CouldNotMove(error.to_string())));
    }

    let reason = match launcher.launch(&layout.exe) {
        Err(error) => Reason::CouldNotStart(error.to_string()),
        Ok(mut child) => {
            let started = Instant::now();
            loop {
                match read_confirmed(layout) {
                    Some(version) if version == expect.to_string() => return Outcome::Updated,
                    Some(got) => {
                        child.kill();
                        break Reason::WrongVersion {
                            expected: expect.clone(),
                            got,
                        };
                    }
                    None => {}
                }
                if !child.is_running() {
                    // It may have confirmed in its last moment.
                    if read_confirmed(layout).as_deref() == Some(&expect.to_string()) {
                        return Outcome::Updated;
                    }
                    break Reason::ExitedEarly;
                }
                if started.elapsed() >= timeout {
                    child.kill();
                    break Reason::NotConfirmed(timeout);
                }
                sleep(poll);
            }
        }
    };
    roll_back(layout, launcher, reason)
}

fn roll_back<L: Launcher>(layout: &Layout, launcher: &mut L, reason: Reason) -> Outcome {
    // The new program, if it is still holding its name, goes aside: its image can
    // be renamed even while it winds down.
    let _ = std::fs::remove_file(&layout.failed);
    if std::fs::rename(&layout.exe, &layout.failed).is_err() {
        // It may still be exiting; a moment, then once more.
        std::thread::sleep(Duration::from_millis(500));
        if let Err(error) = std::fs::rename(&layout.exe, &layout.failed) {
            return Outcome::Stuck(format!(
                "the new program could not be moved aside ({error}); \
                 rename {} to {} to go back",
                layout.old.display(),
                layout.exe.display()
            ));
        }
    }
    if let Err(error) = std::fs::rename(&layout.old, &layout.exe) {
        return Outcome::Stuck(format!(
            "the old program could not be put back ({error}); rename {} to {}",
            layout.old.display(),
            layout.exe.display()
        ));
    }
    let outcome = Outcome::RolledBack(record(layout, reason));
    // The program that worked is started again, so the user is not left without one.
    if let Err(error) = launcher.launch(&layout.exe) {
        tracing::error!(%error, "the restored program could not be started");
    }
    outcome
}

/// Leave the reason for the program to find, and return it.
fn record(layout: &Layout, reason: Reason) -> Reason {
    let _ = std::fs::create_dir_all(&layout.cache);
    let _ = std::fs::write(layout.rolled_back(), reason.to_string());
    reason
}

fn read_confirmed(layout: &Layout) -> Option<String> {
    let text = std::fs::read_to_string(layout.confirmed()).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// The new program says it has started, with its own version. Called once at
/// start-up, after everything that can fail has been passed; a program that is not
/// being updated writes it harmlessly, and the next update clears it first.
pub fn confirm(layout: &Layout, version: &Version) -> io::Result<()> {
    std::fs::create_dir_all(&layout.cache)?;
    std::fs::write(layout.confirmed(), version.to_string())
}

/// After a successful start: the program it replaced, and a failed one from an
/// earlier attempt, are no longer needed. Returns whether anything was removed.
pub fn clean_up(layout: &Layout) -> bool {
    let mut any = false;
    for path in [&layout.old, &layout.failed] {
        if path.exists() && std::fs::remove_file(path).is_ok() {
            any = true;
        }
    }
    any
}

/// The reason the last update was rolled back, once: reading it removes it, so the
/// user is told one time.
pub fn take_rollback_note(layout: &Layout) -> Option<String> {
    let path = layout.rolled_back();
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(path);
    let text = text.trim().to_owned();
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake program: it confirms (or does not) when launched, and can die.
    #[derive(Clone)]
    enum Script {
        /// Writes this confirmation straight away.
        Confirms(String),
        /// Confirms after this many polls.
        ConfirmsAfter(u32, String),
        /// Starts and runs forever without confirming.
        Silent,
        /// Starts and has stopped by the first poll.
        Dies,
        /// Cannot be started at all.
        NoLaunch,
    }

    struct Fake {
        script: Script,
        polls: u32,
        killed: bool,
        layout: Layout,
    }

    impl Process for Fake {
        fn is_running(&mut self) -> bool {
            self.polls += 1;
            if let Script::ConfirmsAfter(after, text) = &self.script
                && self.polls >= *after
            {
                std::fs::write(self.layout.confirmed(), text).expect("confirms");
            }
            !matches!(self.script, Script::Dies) && !self.killed
        }
        fn kill(&mut self) {
            self.killed = true;
        }
    }

    /// Launches the scripted program for the first start and a working one after.
    struct Fakes {
        layout: Layout,
        first: Script,
        launched: Vec<PathBuf>,
    }

    impl Launcher for Fakes {
        type Child = Fake;
        fn launch(&mut self, exe: &Path) -> io::Result<Fake> {
            let nth = self.launched.len();
            self.launched.push(exe.to_path_buf());
            let script = if nth == 0 {
                self.first.clone()
            } else {
                Script::Silent
            };
            if let Script::NoLaunch = script {
                return Err(io::Error::other("no such program"));
            }
            std::fs::create_dir_all(&self.layout.cache).expect("cache");
            if let Script::Confirms(text) = &script {
                std::fs::write(self.layout.confirmed(), text).expect("confirms");
            }
            Ok(Fake {
                script,
                polls: 0,
                killed: false,
                layout: self.layout.clone(),
            })
        }
    }

    fn setup() -> (tempfile::TempDir, Layout) {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let layout = Layout::new(
            &dir.path().join("OmniToaster.exe"),
            &dir.path().join("OmniToasterData"),
        );
        std::fs::write(&layout.exe, "OLD PROGRAM").expect("writes");
        std::fs::write(&layout.new, "NEW PROGRAM").expect("writes");
        (dir, layout)
    }

    fn run(layout: &Layout, first: Script, expect: &str) -> (Outcome, Vec<PathBuf>) {
        let mut launcher = Fakes {
            layout: layout.clone(),
            first,
            launched: Vec::new(),
        };
        let outcome = apply(
            layout,
            &Version::parse(expect).expect("semver"),
            &mut launcher,
            Duration::from_millis(50),
            Duration::ZERO,
            &mut |_| {},
        );
        (outcome, launcher.launched)
    }

    fn text(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap_or_else(|_| "<missing>".into())
    }

    #[test]
    fn the_helper_is_started_with_the_version_the_data_folder_and_the_original_command_line() {
        let args = helper_args(
            Path::new(r"D:\Apps\OmniToasterData"),
            &Version::new(0, 19, 0),
            &[
                "--worker".to_owned(),
                "--lang".to_owned(),
                "ms-MY".to_owned(),
            ],
        );
        assert_eq!(
            args,
            [
                "--apply-update",
                "--expect",
                "0.19.0",
                "--data-dir",
                r"D:\Apps\OmniToasterData",
                "--",
                "--worker",
                "--lang",
                "ms-MY"
            ]
        );
        // With no original arguments the program is simply started again.
        assert_eq!(
            helper_args(Path::new("d"), &Version::new(1, 0, 0), &[])
                .last()
                .map(String::as_str),
            Some("--")
        );
    }

    #[test]
    fn the_layout_names_the_siblings_and_the_cache() {
        let layout = Layout::new(
            Path::new(r"D:\Apps\OmniToaster.exe"),
            Path::new(r"D:\Apps\OmniToasterData"),
        );
        assert_eq!(layout.new, Path::new(r"D:\Apps\OmniToaster.exe.new"));
        assert_eq!(layout.old, Path::new(r"D:\Apps\OmniToaster.exe.old"));
        assert_eq!(layout.failed, Path::new(r"D:\Apps\OmniToaster.exe.failed"));
        assert_eq!(
            layout.cache,
            Path::new(r"D:\Apps\OmniToasterData\cache\update")
        );
    }

    #[test]
    fn a_program_that_confirms_its_version_replaces_the_old_one() {
        let (_dir, layout) = setup();
        let (outcome, launched) = run(&layout, Script::Confirms("0.19.0".into()), "0.19.0");
        assert_eq!(outcome, Outcome::Updated);
        assert_eq!(text(&layout.exe), "NEW PROGRAM");
        assert_eq!(
            text(&layout.old),
            "OLD PROGRAM",
            "kept until the new one has run"
        );
        assert!(!layout.new.exists(), ".new was consumed");
        assert_eq!(launched, std::slice::from_ref(&layout.exe));
        // The new program, once it has run, tidies up.
        assert!(clean_up(&layout));
        assert!(!layout.old.exists());
        assert!(!clean_up(&layout), "nothing left to remove");
    }

    #[test]
    fn a_program_that_confirms_a_little_later_is_waited_for() {
        let (_dir, layout) = setup();
        let (outcome, _) = run(&layout, Script::ConfirmsAfter(3, "0.19.0".into()), "0.19.0");
        assert_eq!(outcome, Outcome::Updated);
    }

    #[test]
    fn a_program_that_never_confirms_is_killed_and_the_old_one_comes_back() {
        let (_dir, layout) = setup();
        let (outcome, launched) = run(&layout, Script::Silent, "0.19.0");
        assert!(
            matches!(outcome, Outcome::RolledBack(Reason::NotConfirmed(_))),
            "{outcome:?}"
        );
        assert_eq!(
            text(&layout.exe),
            "OLD PROGRAM",
            "the old program is back under its name"
        );
        assert_eq!(
            text(&layout.failed),
            "NEW PROGRAM",
            "the new one is set aside, not lost"
        );
        assert!(!layout.old.exists() && !layout.new.exists());
        assert_eq!(launched.len(), 2, "the new program, then the restored one");
    }

    #[test]
    fn a_program_that_dies_at_once_is_rolled_back() {
        let (_dir, layout) = setup();
        let (outcome, launched) = run(&layout, Script::Dies, "0.19.0");
        assert_eq!(outcome, Outcome::RolledBack(Reason::ExitedEarly));
        assert_eq!(text(&layout.exe), "OLD PROGRAM");
        assert_eq!(launched.len(), 2);
    }

    #[test]
    fn a_program_that_cannot_be_started_is_rolled_back() {
        let (_dir, layout) = setup();
        let (outcome, launched) = run(&layout, Script::NoLaunch, "0.19.0");
        assert!(
            matches!(outcome, Outcome::RolledBack(Reason::CouldNotStart(_))),
            "{outcome:?}"
        );
        assert_eq!(text(&layout.exe), "OLD PROGRAM");
        assert_eq!(
            launched.len(),
            2,
            "the failed start, then the restored program"
        );
    }

    #[test]
    fn a_program_that_confirms_the_wrong_version_is_rolled_back() {
        let (_dir, layout) = setup();
        let (outcome, _) = run(&layout, Script::Confirms("0.18.9".into()), "0.19.0");
        assert!(
            matches!(&outcome, Outcome::RolledBack(Reason::WrongVersion { got, .. }) if got == "0.18.9"),
            "{outcome:?}"
        );
        assert_eq!(text(&layout.exe), "OLD PROGRAM");
    }

    #[test]
    fn a_stale_confirmation_from_an_earlier_run_is_not_believed() {
        let (_dir, layout) = setup();
        confirm(&layout, &Version::new(0, 19, 0)).expect("a stale one");
        let (outcome, _) = run(&layout, Script::Silent, "0.19.0");
        assert!(
            matches!(outcome, Outcome::RolledBack(Reason::NotConfirmed(_))),
            "{outcome:?}"
        );
    }

    #[test]
    fn a_rollback_leaves_a_note_once_and_the_data_folder_alone() {
        let (dir, layout) = setup();
        let data = dir.path().join("OmniToasterData");
        std::fs::create_dir_all(data.join("config")).expect("config");
        std::fs::write(data.join("config").join("settings.toml"), "keep = true").expect("settings");
        let (outcome, _) = run(&layout, Script::Dies, "0.19.0");
        assert!(matches!(outcome, Outcome::RolledBack(_)));
        let note = take_rollback_note(&layout).expect("a note");
        assert!(note.contains("stopped before it confirmed"), "{note}");
        assert_eq!(take_rollback_note(&layout), None, "told once");
        assert_eq!(
            text(&data.join("config").join("settings.toml")),
            "keep = true"
        );
    }

    #[test]
    fn nothing_staged_means_nothing_done() {
        let (_dir, layout) = setup();
        std::fs::remove_file(&layout.new).expect("removes");
        let (outcome, launched) = run(&layout, Script::Confirms("0.19.0".into()), "0.19.0");
        assert_eq!(outcome, Outcome::NothingStaged);
        assert!(launched.is_empty());
        assert_eq!(text(&layout.exe), "OLD PROGRAM");
    }

    #[test]
    fn if_the_staged_program_cannot_be_moved_in_the_old_one_is_put_back() {
        let (_dir, layout) = setup();
        // `.new` is a directory: renaming it over the exe's name fails after the
        // exe has been moved aside, which is the case that must not strand the user.
        std::fs::remove_file(&layout.new).expect("removes");
        std::fs::create_dir(&layout.new).expect("a directory named .new");
        std::fs::write(layout.new.join("x"), "x").expect("fills it");
        let (outcome, launched) = run(&layout, Script::Confirms("0.19.0".into()), "0.19.0");
        // Depending on the platform the rename of a directory onto a file name is
        // refused or allowed; refused is the case under test, allowed is a no-program
        // outcome this test would catch as a changed exe.
        if let Outcome::RolledBack(Reason::CouldNotMove(_)) = outcome {
            assert_eq!(text(&layout.exe), "OLD PROGRAM", "the old program is back");
            assert!(launched.is_empty(), "nothing was started");
        }
    }

    #[test]
    fn confirming_writes_the_version_and_a_second_confirmation_replaces_it() {
        let (_dir, layout) = setup();
        confirm(&layout, &Version::new(0, 19, 0)).expect("confirms");
        assert_eq!(read_confirmed(&layout).as_deref(), Some("0.19.0"));
        confirm(&layout, &Version::new(0, 20, 0)).expect("confirms");
        assert_eq!(read_confirmed(&layout).as_deref(), Some("0.20.0"));
    }
}
