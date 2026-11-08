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

mod self_test;

use std::path::PathBuf;
use std::process::ExitCode;

use ot_core::DataRoot;
use ot_core::logging::{self, DEFAULT_RETENTION_DAYS, HostInfo};
use ot_i18n::Language;

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
];

fn main() -> ExitCode {
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
    let language = match value_of(&args, "--lang") {
        Ok(Some(tag)) => Language::from_os_locale(&tag),
        Ok(None) => match ot_platform::user_locale() {
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
        eprintln!("OmniToaster cannot start: {error}");
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
            eprintln!("{}", ot_i18n::text_with(key, &[("path", path.into())]));
            eprintln!("{}", ot_i18n::text("error-data-dir-advice"));
            eprintln!("({error})");
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
            eprintln!("OmniToaster cannot start: {error}");
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
            eprintln!("OmniToaster cannot start: {error}");
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

    tracing::info!("startup complete");
    drop(guard);
    ExitCode::SUCCESS
}

/// Print `--help` in the active language.
///
/// The option names themselves stay in English — they are what the user types
/// — while every description is a Fluent key, so a Malay user gets Malay
/// explanations of English flags, which is what every other CLI does too.
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
