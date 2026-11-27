//! The update helper role, and the two ends of the swap's handshake (FEAT-0079,
//! `docs/specs/updater.md` §5).
//!
//! `OmniToaster.exe --apply-update --expect <version> [--data-dir <d>] -- <args>`
//! is the **old** exe, started by the program that has staged a new one and is
//! about to quit. It waits for that program to be gone (the single-instance names
//! become free), renames the new exe into place, starts it again with `<args>` — the
//! command line the user's copy was started with — and waits for it to confirm. If it
//! does not, the old program is put back (`ot_update::swap`).
//!
//! The other end is [`after_startup`]: the program calls it once it has started,
//! which confirms its version and removes the program it replaced.

use std::path::Path;
use std::process::{Child, ExitCode};
use std::time::{Duration, Instant};

use ot_platform::{InstanceGuard, Role};
use ot_update::swap::{self, Launcher, Layout, Outcome, Process};
use semver::Version;

/// How long the helper waits for the program that staged the update to let go of
/// its single-instance names before it gives up.
const RELEASE_WAIT: Duration = Duration::from_secs(30);

/// A real started program.
struct Running(Child);

impl Process for Running {
    fn is_running(&mut self) -> bool {
        matches!(self.0.try_wait(), Ok(None))
    }

    fn kill(&mut self) {
        let _ = self.0.kill();
    }
}

/// Starts the exe with the command line the copy was started with.
struct Starter {
    args: Vec<String>,
}

impl Launcher for Starter {
    type Child = Running;

    fn launch(&mut self, exe: &Path) -> std::io::Result<Running> {
        ot_platform::process::spawn_hidden(exe, &self.args).map(Running)
    }
}

/// Wait until neither single-instance name is held, so the new program can take it.
fn wait_for_release(timeout: Duration) -> bool {
    let started = Instant::now();
    loop {
        // Held and dropped: all that matters is that it could be taken.
        let free = |role| InstanceGuard::acquire(role).is_ok();
        if free(Role::Supervisor) && free(Role::Worker) {
            return true;
        }
        if started.elapsed() >= timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Append a line to the helper's own log. The helper starts no logging of its
/// own, which would share the day's file with the program it is starting.
fn note(cache: &Path, line: &str) {
    use std::io::Write as _;
    let _ = std::fs::create_dir_all(cache);
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(cache.join("helper.log"))
    {
        let _ = writeln!(file, "{line}");
    }
}

/// What follows `--` on the command line: the arguments to start the program with.
fn passed_through(args: &[String]) -> Vec<String> {
    args.iter()
        .position(|a| a == "--")
        .map(|at| args[at + 1..].to_vec())
        .unwrap_or_default()
}

/// Run the helper. `data_dir` is the data folder the copy uses.
pub fn run(args: &[String], data_dir: &Path, expect: &str) -> ExitCode {
    let Ok(expect) = Version::parse(expect) else {
        return ExitCode::FAILURE;
    };
    let Ok(exe) = std::env::current_exe() else {
        return ExitCode::FAILURE;
    };
    let layout = Layout::new(&exe, data_dir);
    note(
        &layout.cache,
        &format!("apply {expect}: waiting for the program to let go"),
    );
    if !wait_for_release(RELEASE_WAIT) {
        note(
            &layout.cache,
            "the program did not let go in time; nothing was changed",
        );
        return ExitCode::FAILURE;
    }
    let mut starter = Starter {
        args: passed_through(args),
    };
    let outcome = swap::apply(
        &layout,
        &expect,
        &mut starter,
        confirm_timeout(),
        Duration::from_millis(250),
        &mut std::thread::sleep,
    );
    note(&layout.cache, &format!("outcome: {outcome:?}"));
    match outcome {
        Outcome::Updated | Outcome::NothingStaged => ExitCode::SUCCESS,
        Outcome::RolledBack(_) | Outcome::Stuck(_) => ExitCode::FAILURE,
    }
}

/// The confirmation timeout: [`swap::CONFIRM_TIMEOUT`], or a debug build's
/// `OMNITOASTER_CONFIRM_TIMEOUT_MS` so the tests need not wait thirty seconds.
fn confirm_timeout() -> Duration {
    #[cfg(debug_assertions)]
    if let Some(ms) = std::env::var("OMNITOASTER_CONFIRM_TIMEOUT_MS")
        .ok()
        .and_then(|text| text.parse::<u64>().ok())
    {
        return Duration::from_millis(ms);
    }
    swap::CONFIRM_TIMEOUT
}

/// The program has started: say so, with its version, and remove the program an
/// update replaced. A copy that was not just updated writes a confirmation nobody
/// reads and finds nothing to remove.
pub fn after_startup(data_dir: &Path, version: &str) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let layout = Layout::new(&exe, data_dir);
    if let Ok(version) = Version::parse(version)
        && let Err(error) = swap::confirm(&layout, &version)
    {
        tracing::warn!(%error, "the start-up confirmation could not be written");
    }
    if swap::clean_up(&layout) {
        tracing::info!("removed the program the last update replaced");
    }
}
