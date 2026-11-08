//! `--self-test`: the headless smoke check CI runs on the built exe.
//!
//! The contract (`docs/09-ci-cd-and-releases.md`) is one line per check,
//! **flushed as it prints**, and exit code 0 only when every line said `ok`.
//! The flushing is the whole point: a buffered self-test that dies mid-check
//! tells you nothing about which check died, which `docs/GOTCHAS.md` records
//! as a lesson paid for once already.
//!
//! The release profile sets `panic = "abort"`, so a panic inside a check
//! cannot be caught and turned into a `FAILED` line — `catch_unwind` does not
//! work without unwinding. That is deliberate (the supervisor wants a
//! non-zero exit, not a half-unwound process), so the check name is printed
//! **before** the check runs and the verdict after it. An abort therefore
//! leaves the name of the check that died as the last line of output, which
//! gets the same diagnostic value a different way.

use std::io::Write as _;
use std::path::Path;

use ot_core::DataRoot;
use ot_core::logging::{self, DEFAULT_RETENTION_DAYS};
use ot_i18n::{Language, TimePrefs};

/// Run every check against `data_dir`, printing one line each.
///
/// Returns the number of failures, so the caller can map it to an exit code.
pub fn run(data_dir: Option<&Path>) -> usize {
    let mut failures = 0;

    // Each check returns Err(reason) rather than panicking, so one failure
    // does not stop the rest: a self-test that stops at the first problem
    // hides the others.
    failures += check("paths resolve and are writable", || {
        let root = DataRoot::resolve(data_dir).map_err(|e| e.to_string())?;
        for sub in ["config", "data", "logs", "cache"] {
            let path = root.join(sub).map_err(|e| e.to_string())?;
            if !path.is_dir() {
                return Err(format!("{sub} was not created"));
            }
        }
        Ok(())
    });

    failures += check("the log file is written", || {
        let root = DataRoot::resolve(data_dir).map_err(|e| e.to_string())?;
        // Logging may already be initialised by the caller; either way what
        // matters is that a line reaches a file on disk.
        let guard = logging::init(&root, "info", DEFAULT_RETENTION_DAYS).ok();
        tracing::info!("self-test");
        drop(guard);

        let logs = root.join("logs").map_err(|e| e.to_string())?;
        let any = std::fs::read_dir(&logs)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .any(|entry| {
                entry.file_name().to_string_lossy().starts_with("app")
                    && entry.metadata().map(|m| m.len() > 0).unwrap_or(false)
            });
        if any {
            Ok(())
        } else {
            Err(format!("no non-empty app log in {}", logs.display()))
        }
    });

    failures += check("both locale bundles parse", || {
        for language in [Language::English, Language::Malay] {
            ot_i18n::probe_language(language).map_err(|e| format!("{}: {e}", language.tag()))?;
        }
        Ok(())
    });

    failures += check("locale key parity holds", ot_i18n::check_key_parity);

    failures += check("the database migrates", || {
        let root = DataRoot::resolve(data_dir).map_err(|e| e.to_string())?;
        let store = ot_store::Store::open(root.path()).map_err(|e| e.to_string())?;
        let version = store.schema_version().map_err(|e| e.to_string())?;
        if version >= 1 {
            Ok(())
        } else {
            Err(format!("schema version is {version}, expected at least 1"))
        }
    });

    failures += check("the date formatter answers in both languages", || {
        let when = chrono_now();
        for language in [Language::English, Language::Malay] {
            // The global loader holds one language at a time, so this checks
            // the formatter produces something non-empty with the prefs each
            // language implies rather than comparing exact words.
            let rendered = ot_i18n::format_datetime(&when, TimePrefs::for_language(language));
            if rendered.trim().is_empty() {
                return Err(format!("{} produced an empty string", language.tag()));
            }
            if rendered.contains('\u{2068}') || rendered.contains('\u{2069}') {
                return Err(format!("{} leaked Fluent isolate marks", language.tag()));
            }
        }
        Ok(())
    });

    let total = 6;
    if failures == 0 {
        println!("{total} checks, all ok");
    } else {
        println!("{total} checks, {failures} FAILED");
    }
    let _ = std::io::stdout().flush();
    failures
}

/// Print the check's name, run it, print the verdict — each flushed.
fn check(name: &str, body: impl FnOnce() -> Result<(), String>) -> usize {
    // The name goes out before the work, so an abort leaves it as the last
    // line and names the check that died.
    print!("{name} ... ");
    let _ = std::io::stdout().flush();

    match body() {
        Ok(()) => {
            println!("ok");
            let _ = std::io::stdout().flush();
            0
        }
        Err(reason) => {
            println!("FAILED: {reason}");
            let _ = std::io::stdout().flush();
            1
        }
    }
}

/// Now, as a naive local time. Kept here so the formatter check does not need
/// chrono as a direct dependency of this crate's public surface.
fn chrono_now() -> chrono::NaiveDateTime {
    chrono::Local::now().naive_local()
}
