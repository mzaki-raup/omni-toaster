//! OmniToaster — portable Windows toast notifications.
//!
//! One executable, several roles, chosen by the command line
//! (`docs/specs/supervisor.md`):
//!
//! - no flag — the supervisor, which spawns and watches the worker
//! - `--worker` — the application itself: tray, settings, toasts, scheduler
//! - `--self-test` — the headless smoke check CI runs
//! - `--data-dir <path>` — use this folder instead of `OmniToasterData/`
//!
//! `--worker` is accepted and recognised but does nothing yet: the supervisor
//! that would spawn it is Phase 8. Everything else here is live.

// A release build is a GUI program: no console window behind the tray icon
// (BUG-0013). A debug build keeps its console, where `cargo run` shows the log.
// `console::attach` gives a terminal its output back and `console::report`
// shows a startup failure in a message box, so neither is lost with it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod self_test;

use std::path::PathBuf;
use std::process::ExitCode;

use ot_core::DataRoot;
use ot_core::logging::{self, DEFAULT_RETENTION_DAYS, HostInfo};
use ot_i18n::Language;
use ot_platform::{InstanceGuard, Role};

/// Printed by `--version` and compared against the tag being released, so a
/// binary built from the wrong `Cargo.toml` cannot be packaged.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Every option the binary accepts. An argument starting `--` that is not one
/// of these is a mistake worth naming rather than ignoring: a silently dropped
/// `--data-dir` typo would write into the real data folder.
const KNOWN_FLAGS: &[&str] = &[
    "--version",
    "--help",
    "--self-test",
    "--worker",
    "--data-dir",
    "--log-level",
    "--lang",
    #[cfg(debug_assertions)]
    "--panic-now",
    // Holds the single-instance guard so a second launch overlaps with the
    // first. Until FEAT-0014 there is no window to keep the process alive, so
    // without this the first copy exits before a second can be started and
    // the guard cannot be tested across processes at all. Debug only.
    #[cfg(debug_assertions)]
    "--hold-ms",
    // Fires one toast at startup, so the renderer can be looked at without a
    // tray click. The tray's Test Toast is the real trigger; this is the one
    // a session with no mouse on the notification area can reach. Debug only.
    #[cfg(debug_assertions)]
    "--test-toast",
    // Scales the UI as a larger display scaling would, for checking a layout
    // at 150% and 200% without changing the machine's own display settings.
    // Debug only.
    #[cfg(debug_assertions)]
    "--ui-scale",
    // Every toast fades, as with Windows' Show animations off, for checking
    // reduced motion without changing the machine's settings. Debug only.
    #[cfg(debug_assertions)]
    "--reduce-motion",
    // Every frame counts as 50 ms, forcing the frame-time fallback, for
    // checking it on a machine fast enough never to need it. Debug only.
    #[cfg(debug_assertions)]
    "--slow-frames",
    // Creates a reminder that many seconds out, and with a second number
    // edits it ten seconds later to that many seconds after the start, for
    // checking the scheduler before Settings ▸ Reminders exists. Debug only.
    #[cfg(debug_assertions)]
    "--test-reminder",
    // Shortens the ten-minute cap on an alarm's toast to this many seconds,
    // for watching it close at the cap without waiting ten minutes. Debug
    // only.
    #[cfg(debug_assertions)]
    "--alarm-cap",
    // A weather code to simulate: the weather service reads a sky that is clear
    // once and then this code, every few seconds, so a change of weather can be
    // watched without waiting for rain. Debug only.
    #[cfg(debug_assertions)]
    "--simulate-weather",
];

fn main() -> ExitCode {
    ot_platform::console::attach();
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("OmniToaster {VERSION}");
        return ExitCode::SUCCESS;
    }

    if let Some(unknown) = args.iter().find(|a| {
        a.starts_with("--")
            && !KNOWN_FLAGS
                .iter()
                .any(|known| *a == known || a.starts_with(&format!("{known}=")))
    }) {
        // Naming it beats a generic usage dump: a typo'd flag is the common
        // case and the user wants to know which one.
        eprintln!("OmniToaster does not know the option {unknown}.");
        eprintln!("Run OmniToaster --help to see the options it does know.");
        return ExitCode::FAILURE;
    }

    let data_dir = match data_dir_from(&args) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };

    // Localization comes up before anything that can fail with a message,
    // because the startup errors are the only strings a user can see before
    // the settings window exists and they must be in their own language.
    // --lang wins, then the user's stored choice, then Windows. The stored
    // choice is read straight out of the data folder because the folder has
    // not been validated yet and its failure messages are themselves
    // localized (ot_core::settings::stored_language).
    let language = match value_of(&args, "--lang") {
        Ok(Some(tag)) => Language::from_os_locale(&tag),
        Ok(None) => match ot_core::settings::stored_language(data_dir.as_deref())
            .or_else(ot_platform::user_locale)
        {
            Some(tag) => Language::from_os_locale(&tag),
            None => Language::English,
        },
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = ot_i18n::init(language) {
        // Nothing is translatable yet if this failed, so plain English is all
        // there is.
        fail(&format!("OmniToaster cannot start: {error}"));
        return ExitCode::FAILURE;
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return ExitCode::SUCCESS;
    }

    if args.iter().any(|a| a == "--self-test") {
        // Runs its own path resolution, so it is reached before the normal
        // startup: the point is to test those steps, not to inherit them.
        let failures = self_test::run(data_dir.as_deref());
        return if failures == 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    // One instance per role. Taken after --self-test and --help, which are
    // one-shot and must work while the app is already running, and before
    // anything that touches the data folder — two processes migrating the same
    // database is exactly what this prevents.
    let role = if args.iter().any(|a| a == "--worker") {
        Role::Worker
    } else {
        Role::Supervisor
    };
    let _instance = match InstanceGuard::acquire(role) {
        Ok(guard) => guard,
        Err(ot_platform::InstanceError::AlreadyRunning) => {
            // Not a failure: the user asked for OmniToaster and OmniToaster is
            // running. Focusing the existing window is FEAT-0014's job.
            ot_platform::console::report(
                ot_platform::console::Kind::Notice,
                &ot_i18n::text("notice-already-running"),
            );
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            fail(&format!("OmniToaster cannot start: {error}"));
            return ExitCode::FAILURE;
        }
    };

    let root = match DataRoot::resolve(data_dir.as_deref()) {
        Ok(root) => root,
        Err(error) => {
            // The typed error picks the message; the cause goes on its own
            // line so the user sees advice first and detail second.
            let key = match &error {
                ot_core::PathError::NotWritable { .. } => "error-data-dir-not-writable",
                ot_core::PathError::NotCreatable { .. } => "error-data-dir-not-creatable",
                ot_core::PathError::ExeLocationUnknown { .. } => "error-exe-location-unknown",
                ot_core::PathError::Escapes { .. } => "error-data-dir-not-writable",
            };
            let path = data_dir
                .as_deref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| ot_core::paths::DATA_DIR_NAME.to_string());
            fail(&format!(
                "{}
{}
({error})",
                ot_i18n::text_with(key, &[("path", path.into())]),
                ot_i18n::text("error-data-dir-advice"),
            ));
            return ExitCode::FAILURE;
        }
    };

    let level = match value_of(&args, "--log-level") {
        Ok(Some(level)) => level,
        Ok(None) => "info".to_string(),
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };

    // The guard must outlive everything that logs: dropping it stops the
    // background writer, so the last lines before an exit would be lost.
    let guard = match logging::init(&root, &level, DEFAULT_RETENTION_DAYS) {
        Ok(guard) => guard,
        Err(error) => {
            fail(&format!("OmniToaster cannot start: {error}"));
            return ExitCode::FAILURE;
        }
    };

    logging::install_panic_hook(&root, guard.recent(), host_info());

    // The database is opened after logging, so a migration failure is in the
    // log file rather than only on a console nobody is watching.
    let store = match ot_store::Store::open(root.path()) {
        Ok(store) => store,
        Err(error) => {
            tracing::error!(%error, "the database could not be opened");
            fail(&format!("OmniToaster cannot start: {error}"));
            return ExitCode::FAILURE;
        }
    };

    println!("OmniToaster {VERSION}");
    println!("Language:    {}", ot_i18n::text(language.name_key()));
    println!("Data folder: {}", root.path().display());
    println!(
        "{}",
        ot_i18n::text_with(
            "notice-log-location",
            &[(
                "path",
                root.path().join("logs").display().to_string().into()
            )],
        )
    );

    // A debug-only trigger, so the panic hook can be exercised in the running
    // app without waiting for a real crash. Never compiled into a release.
    #[cfg(debug_assertions)]
    if args.iter().any(|a| a == "--panic-now") {
        panic!("deliberate panic from --panic-now");
    }

    match store.schema_version() {
        Ok(version) => println!("Database:    schema version {version}"),
        Err(error) => tracing::warn!(%error, "the schema version could not be read"),
    }

    // History older than 90 days goes on every start
    // (docs/05-data-model-and-storage.md). A failure costs disk space, not
    // correctness, so it is logged and the app carries on.
    match store.prune_notification_log(jiff::Timestamp::now()) {
        Ok(0) => {}
        Ok(pruned) => tracing::info!(pruned, "old notification history pruned"),
        Err(error) => tracing::warn!(%error, "the notification history could not be pruned"),
    }

    // Refresh the autostart path if it is on and the folder has moved. Never
    // enables it: that is the user's choice, not a side effect of launching
    // (docs/specs/tray-and-settings.md §7).
    match ot_platform::autostart::refresh_if_enabled() {
        Ok(true) => {
            tracing::info!("autostart pointed at an old path and was refreshed");
            println!("Autostart:   refreshed to this folder");
        }
        Ok(false) => match ot_platform::autostart::status() {
            Ok(state) => println!("Autostart:   {}", if state.is_on() { "on" } else { "off" }),
            Err(error) => tracing::warn!(%error, "the autostart state could not be read"),
        },
        Err(error) => tracing::warn!(%error, "autostart could not be refreshed"),
    }

    tracing::info!("startup complete");

    // Hold the instance guard without a window, so a second launch can be
    // observed being refused and so the command line can be exercised where
    // no window can open at all — CI runs as a service, in session 0. Debug
    // only, and it is what keeps every test in tests/cli.rs headless.
    #[cfg(debug_assertions)]
    if let Ok(Some(text)) = value_of(&args, "--hold-ms")
        && let Ok(ms) = text.parse::<u64>()
    {
        println!("Holding:     {ms} ms");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        std::thread::sleep(std::time::Duration::from_millis(ms));
        drop(guard);
        return ExitCode::SUCCESS;
    }

    #[cfg(debug_assertions)]
    let test_reminder = match value_of(&args, "--test-reminder") {
        Ok(Some(spec)) => test_reminder(&store, &spec),
        _ => None,
    };

    // Read before the scheduler starts: it needs the catch-up window for the
    // reminders missed while the app was not running.
    let settings = match ot_core::Settings::load(&root) {
        Ok(settings) => settings,
        Err(error) => {
            // The file is the user's to edit, so a typo in it must not lock
            // them out of the app that would let them fix it.
            tracing::warn!(%error, "the settings file could not be read; using the defaults");
            eprintln!("({error})");
            ot_core::Settings::default()
        }
    };

    // The managed prayer reminders exist before the scheduler reads the store,
    // so a prayer that came due while the app was off is found and found missed
    // (docs/11-prayer-and-azan.md §9).
    if let Err(error) = store.sync_prayer_reminders(&settings.prayer.enabled_prayers()) {
        tracing::warn!(%error, "the prayer reminders could not be set up");
    }

    // The scheduler takes the store: from here on only its thread writes fire
    // times (docs/specs/reminders.md §12).
    let (fire_out, fires) = ot_sched::scheduler::fire_channel();
    let scheduler = match ot_sched::scheduler::Scheduler::spawn(
        store,
        fire_out,
        settings.reminders.clamped().missed_window_hours,
    ) {
        Ok(scheduler) => scheduler,
        Err(error) => {
            tracing::error!(%error, "the scheduler could not be started");
            fail(&format!("OmniToaster cannot start: {error}"));
            return ExitCode::FAILURE;
        }
    };
    let commands = scheduler.commands();
    let _system_events = match ot_platform::system_events::watch(move |event| {
        use ot_sched::scheduler::{Command, Recompute};
        let why = match event {
            ot_platform::SystemEvent::TimeChanged => Recompute::ClockChanged,
            ot_platform::SystemEvent::Resumed => Recompute::Resumed,
        };
        let _ = commands.send(Command::Recompute(why));
    }) {
        Ok(watcher) => Some(watcher),
        Err(error) => {
            // Not fatal: the scheduler notices a clock jump itself and
            // re-reads the zone on every wake; this only makes it immediate.
            tracing::warn!(%error, "clock-change and resume events could not be watched");
            None
        }
    };

    #[cfg(debug_assertions)]
    if let Some((id, Some(moved_to))) = test_reminder {
        let path = root.path().to_path_buf();
        let commands = scheduler.commands();
        let _ = std::thread::Builder::new()
            .name("test-reminder".into())
            .spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(10));
                move_test_reminder(&path, id, moved_to, &commands);
            });
    }

    // The settings window. It blocks until the user closes it, which for now
    // ends the process: Quit belongs to the tray menu (FEAT-0015), and until
    // that exists there would be no way to stop a process that outlived its
    // only window (docs/specs/tray-and-settings.md §6).

    #[cfg(debug_assertions)]
    let test_toast = args.iter().any(|a| a == "--test-toast");
    #[cfg(not(debug_assertions))]
    let test_toast = false;

    #[cfg(debug_assertions)]
    let ui_scale = value_of(&args, "--ui-scale")
        .ok()
        .flatten()
        .and_then(|text| text.parse::<f32>().ok());
    #[cfg(not(debug_assertions))]
    let ui_scale = None;

    #[cfg(debug_assertions)]
    let (reduce_motion, slow_frames) = (
        args.iter().any(|a| a == "--reduce-motion"),
        args.iter().any(|a| a == "--slow-frames"),
    );
    #[cfg(not(debug_assertions))]
    let (reduce_motion, slow_frames) = (false, false);

    #[cfg(debug_assertions)]
    let alarm_cap_secs = value_of(&args, "--alarm-cap")
        .ok()
        .flatten()
        .and_then(|text| text.parse::<f32>().ok());
    #[cfg(not(debug_assertions))]
    let alarm_cap_secs = None;

    #[cfg(debug_assertions)]
    let simulate_weather = value_of(&args, "--simulate-weather")
        .ok()
        .flatten()
        .and_then(|text| text.parse::<u8>().ok());
    #[cfg(not(debug_assertions))]
    let simulate_weather = None;

    let dev = ot_ui::settings_window::DevOptions {
        test_toast,
        ui_scale,
        reduce_motion,
        slow_frames,
        alarm_cap_secs,
        simulate_weather,
    };
    if let Err(error) =
        ot_ui::settings_window::run(root, settings, dev, fires, scheduler.commands())
    {
        tracing::error!(%error, "the settings window could not be opened");
        fail(&format!("OmniToaster cannot show its window: {error}"));
        drop(guard);
        return ExitCode::FAILURE;
    }

    drop(guard);
    ExitCode::SUCCESS
}

/// `--test-reminder <seconds>[,<seconds>]`: a one-off reminder that many
/// seconds from now, and, with the second number, the instant it is to be
/// moved to. Returns its id and that instant.
#[cfg(debug_assertions)]
fn test_reminder(store: &ot_store::Store, spec: &str) -> Option<(i64, Option<jiff::Timestamp>)> {
    use ot_store::reminders::{ReminderDraft, Trigger};

    let now = jiff::Timestamp::now();
    let mut seconds = spec.split(',').map(|part| part.trim().parse::<i64>().ok());
    let at = |secs: i64| now.checked_add(jiff::SignedDuration::from_secs(secs)).ok();
    let first = at(seconds.next().flatten()?)?;
    let moved_to = seconds.next().flatten().and_then(at);

    let draft = ReminderDraft {
        category_id: None,
        // User text, stored as typed: not a Fluent key (docs/10-i18n.md).
        title: "Test reminder".to_owned(),
        body: None,
        authored_lang: "en-US".to_owned(),
        trigger: Trigger::At {
            at: first
                .to_zoned_datetime(jiff::tz::TimeZone::system())
                .datetime(),
        },
        recurrence: None,
    };
    match store.create_reminder(&draft) {
        Ok(id) => {
            tracing::info!(reminder = id, target = %first, "test reminder created");
            println!("Test reminder: {id} at {first}");
            Some((id, moved_to))
        }
        Err(error) => {
            tracing::warn!(%error, "the test reminder could not be created");
            None
        }
    }
}

/// Move the test reminder to `to`, as the editor will: write the row through a
/// connection of its own, then tell the scheduler.
#[cfg(debug_assertions)]
fn move_test_reminder(
    root: &std::path::Path,
    id: i64,
    to: jiff::Timestamp,
    commands: &std::sync::mpsc::Sender<ot_sched::scheduler::Command>,
) {
    let result = ot_store::Store::open(root)
        .map_err(|e| e.to_string())
        .and_then(|store| {
            let mut reminder = store
                .reminder(id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "it is gone".to_owned())?;
            reminder.draft.trigger = ot_store::reminders::Trigger::At {
                at: to
                    .to_zoned_datetime(jiff::tz::TimeZone::system())
                    .datetime(),
            };
            store
                .update_reminder(id, &reminder.draft)
                .map_err(|e| e.to_string())
        });
    match result {
        Ok(_) => {
            tracing::info!(reminder = id, target = %to, "test reminder moved");
            let _ = commands.send(ot_sched::scheduler::Command::Changed(id));
        }
        Err(error) => tracing::warn!(%error, "the test reminder could not be moved"),
    }
}

/// Print `--help` in the active language.
///
/// The option names themselves stay in English — they are what the user types
/// — while every description is a Fluent key, so a Malay user gets Malay
/// explanations of English flags, which is what every other CLI does too.
/// Say why the app is stopping: on the console when there is one, and in a
/// message box when there is not, as in a release build started by a double
/// click (BUG-0013).
fn fail(message: &str) {
    ot_platform::console::report(ot_platform::console::Kind::Error, message);
}

fn print_help() {
    println!("{}", ot_i18n::text("help-usage"));
    println!();
    println!("{}", ot_i18n::text("help-intro"));
    println!();

    // Widest option string, so the descriptions line up in both languages
    // without hard-coding a column that Malay would overflow.
    let options = [
        ("-h, --help", "help-option-help"),
        ("-V, --version", "help-option-version"),
        ("    --self-test", "help-option-self-test"),
        ("    --worker", "help-option-worker"),
        ("    --data-dir <folder>", "help-option-data-dir"),
        ("    --log-level <level>", "help-option-log-level"),
        ("    --lang <tag>", "help-option-lang"),
    ];
    let width = options
        .iter()
        .map(|(flag, _)| flag.chars().count())
        .max()
        .unwrap_or(0);

    for (flag, key) in options {
        println!("  {flag:<width$}  {}", ot_i18n::text(key));
    }

    println!();
    println!("{}", ot_i18n::text("help-footer"));
}

/// What the crash report records about the machine.
///
/// Reading the real Windows build number is platform code and belongs in
/// `ot-platform`, which Phase 1 brings in; until then this is honest about
/// what it does and does not know.
fn host_info() -> HostInfo {
    HostInfo {
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
    }
}

/// Pull `--data-dir <path>` out of the arguments.
///
/// Returns `Ok(None)` when the flag is absent, which means "use
/// `OmniToasterData/` beside the exe".
fn data_dir_from(args: &[String]) -> Result<Option<PathBuf>, String> {
    Ok(value_of(args, "--data-dir")?.map(PathBuf::from))
}

/// Read `--flag <value>` or `--flag=<value>`.
///
/// A flag present with no value is an error rather than a silent default. For
/// `--data-dir` that matters a great deal: defaulting would write into the
/// real `OmniToasterData/` on a typo.
fn value_of(args: &[String], flag: &str) -> Result<Option<String>, String> {
    let missing = || format!("{flag} needs a value, for example {flag} <value>");
    let inline = format!("{flag}=");

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == flag {
            return match iter.next() {
                Some(value) if !value.starts_with("--") => Ok(Some(value.clone())),
                _ => Err(missing()),
            };
        }
        if let Some(value) = arg.strip_prefix(&inline) {
            if value.is_empty() {
                return Err(missing());
            }
            return Ok(Some(value.to_string()));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{VERSION, data_dir_from, value_of};
    use std::path::PathBuf;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn version_is_populated_from_the_workspace_manifest() {
        // The release pipeline matches this value against the tag it is
        // publishing; an empty or placeholder version would let every release
        // offer itself as an update for ever.
        assert!(!VERSION.is_empty(), "CARGO_PKG_VERSION is empty");
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "expected three parts, got {VERSION}");
        for part in parts {
            let numeric = part.split('-').next().unwrap_or_default();
            assert!(
                !numeric.is_empty() && numeric.chars().all(|c| c.is_ascii_digit()),
                "version component {part:?} in {VERSION} is not numeric"
            );
        }
    }

    #[test]
    fn absent_flag_means_the_folder_beside_the_exe() {
        assert_eq!(data_dir_from(&args(&[])).expect("ok"), None);
        assert_eq!(data_dir_from(&args(&["--worker"])).expect("ok"), None);
    }

    #[test]
    fn both_spellings_of_the_flag_are_accepted() {
        let expected = Some(PathBuf::from("./tmp-data"));
        assert_eq!(
            data_dir_from(&args(&["--data-dir", "./tmp-data"])).expect("ok"),
            expected
        );
        assert_eq!(
            data_dir_from(&args(&["--data-dir=./tmp-data"])).expect("ok"),
            expected
        );
    }

    #[test]
    fn a_flag_without_a_folder_is_an_error_not_a_fallback() {
        // Falling back to the real OmniToasterData/ here would be the worst
        // outcome: a developer who fat-fingers the flag would write into it.
        for attempt in [
            args(&["--data-dir"]),
            args(&["--data-dir", "--worker"]),
            args(&["--data-dir="]),
        ] {
            let error = data_dir_from(&attempt).expect_err("must be refused");
            assert!(error.contains("--data-dir needs a value"), "{error}");
        }
    }

    #[test]
    fn log_level_shares_the_same_parsing() {
        assert_eq!(
            value_of(&args(&["--log-level", "debug"]), "--log-level").expect("ok"),
            Some("debug".to_string())
        );
        assert_eq!(
            value_of(&args(&["--log-level=trace"]), "--log-level").expect("ok"),
            Some("trace".to_string())
        );
        assert_eq!(
            value_of(&args(&["--worker"]), "--log-level").expect("ok"),
            None
        );
        let error = value_of(&args(&["--log-level"]), "--log-level").expect_err("refused");
        assert!(error.contains("--log-level needs a value"), "{error}");
    }

    #[test]
    fn every_flag_the_help_lists_is_a_known_flag() {
        // A flag documented in --help but missing from KNOWN_FLAGS would be
        // rejected as unknown the moment someone copied it out of the help
        // text, which is the worst possible place to disagree with yourself.
        for documented in [
            "--help",
            "--version",
            "--self-test",
            "--worker",
            "--data-dir",
            "--log-level",
            "--lang",
        ] {
            assert!(
                super::KNOWN_FLAGS.contains(&documented),
                "{documented} is in --help but not in KNOWN_FLAGS"
            );
        }
    }

    #[test]
    fn the_flag_is_found_after_other_arguments() {
        assert_eq!(
            data_dir_from(&args(&["--worker", "--data-dir", "./scratch"])).expect("ok"),
            Some(PathBuf::from("./scratch"))
        );
    }
}
