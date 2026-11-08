//! Omni Toaster — portable Windows toast notifications.
//!
//! One executable, several roles, chosen by the command line
//! (`docs/specs/supervisor.md`):
//!
//! - no flag — the supervisor, which spawns and watches the worker
//! - `--worker` — the application itself: tray, settings, toasts, scheduler
//! - `--self-test` — the headless smoke check CI runs
//! - `--data-dir <path>` — use this folder instead of `OmniToasterData/`
//!
//! Only `--version` and `--data-dir` do anything yet. FEAT-0006 owns the real
//! command line, including `--self-test`, `--worker` and a bilingual `--help`;
//! `--data-dir` is handled here because FEAT-0002's data root cannot otherwise
//! be exercised in the running app, which is what the project rules require
//! before a feature is called done.

use std::path::PathBuf;
use std::process::ExitCode;

use ot_core::DataRoot;

/// Printed by `--version` and compared against the tag being released, so a
/// binary built from the wrong `Cargo.toml` cannot be packaged.
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("Omni Toaster {VERSION}");
        return ExitCode::SUCCESS;
    }

    let data_dir = match data_dir_from(&args) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };

    // The user-facing wording here is deliberately plain English for now.
    // FEAT-0004 brings Fluent in and these become `error-` keys, so the
    // writability failure a user actually hits is bilingual.
    match DataRoot::resolve(data_dir.as_deref()) {
        Ok(root) => {
            println!("Omni Toaster {VERSION}");
            println!("Data folder: {}", root.path().display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Omni Toaster cannot start: {error}");
            eprintln!(
                "Move the app to a folder you can write to, or pass --data-dir with one. \
                 Nothing was changed."
            );
            ExitCode::FAILURE
        }
    }
}

/// Pull `--data-dir <path>` out of the arguments.
///
/// Returns `Ok(None)` when the flag is absent, which means "use
/// `OmniToasterData/` beside the exe".
fn data_dir_from(args: &[String]) -> Result<Option<PathBuf>, String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--data-dir" {
            return match iter.next() {
                Some(value) if !value.starts_with("--") => Ok(Some(PathBuf::from(value))),
                // A flag with no value is a mistake worth naming, not a
                // silent fallback to the real data folder.
                _ => Err("--data-dir needs a folder, for example --data-dir ./tmp-data".into()),
            };
        }
        if let Some(value) = arg.strip_prefix("--data-dir=") {
            if value.is_empty() {
                return Err("--data-dir needs a folder, for example --data-dir ./tmp-data".into());
            }
            return Ok(Some(PathBuf::from(value)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{VERSION, data_dir_from};
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
            assert!(error.contains("--data-dir needs a folder"), "{error}");
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
