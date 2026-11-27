//! The update helper against real copies of the built exe (FEAT-0079).
//!
//! A unit test cannot show that Windows lets a running exe be renamed and replaced,
//! or that the helper really starts what it swapped in, so these copy the binary
//! under test into a scratch folder as the "installed" exe and as the "staged" one.
//! `--hold-ms` (debug builds) keeps the started copy alive long enough to confirm.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard};

/// The single-instance names are machine-wide, so the copies these tests start
/// would refuse each other if the tests ran side by side.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn built() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_OmniToaster"))
}

struct Install {
    _serial: MutexGuard<'static, ()>,
    _temp: tempfile::TempDir,
    exe: PathBuf,
    new: PathBuf,
    old: PathBuf,
    data: PathBuf,
}

impl Install {
    /// An installed exe, and a staged `.new` that is a copy of it (or garbage).
    fn new(staged_is_garbage: bool) -> Self {
        let serial = ONE_AT_A_TIME
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let temp = tempfile::tempdir().expect("temp dir");
        let exe = temp.path().join("OmniToaster.exe");
        let new = temp.path().join("OmniToaster.exe.new");
        let old = temp.path().join("OmniToaster.exe.old");
        let data = temp.path().join("OmniToasterData");
        std::fs::copy(built(), &exe).expect("copy installed");
        if staged_is_garbage {
            std::fs::write(&new, b"not a program").expect("write garbage");
        } else {
            std::fs::copy(built(), &new).expect("copy staged");
        }
        Self {
            _serial: serial,
            _temp: temp,
            exe,
            new,
            old,
            data,
        }
    }

    fn helper(&self, expect: &str) -> Output {
        Command::new(&self.exe)
            .args(["--apply-update", "--expect", expect, "--data-dir"])
            .arg(&self.data)
            .args(["--", "--data-dir"])
            .arg(&self.data)
            .args(["--hold-ms", "1500"])
            .env("OMNITOASTER_CONFIRM_TIMEOUT_MS", "6000")
            .output()
            .expect("run the helper")
    }

    fn cache(&self, name: &str) -> PathBuf {
        self.data.join("cache").join("update").join(name)
    }
}

#[test]
fn a_staged_exe_is_swapped_in_started_and_confirmed() {
    let install = Install::new(false);
    let before = std::fs::read(&install.exe).expect("read");
    let out = install.helper(env!("CARGO_PKG_VERSION"));
    assert!(out.status.success(), "helper failed: {out:?}");

    assert!(
        !install.new.exists(),
        "the staged file was moved into place"
    );
    assert_eq!(std::fs::read(&install.exe).expect("read"), before);
    assert_eq!(
        std::fs::read_to_string(install.cache("confirmed")).expect("confirmation"),
        env!("CARGO_PKG_VERSION")
    );
    let log = std::fs::read_to_string(install.cache("helper.log")).expect("helper log");
    assert!(log.contains("Updated"), "{log}");
}

#[test]
fn a_staged_exe_that_will_not_start_is_rolled_back() {
    let install = Install::new(true);
    let out = install.helper(env!("CARGO_PKG_VERSION"));
    assert!(!out.status.success(), "a failed update must fail: {out:?}");

    assert_eq!(
        std::fs::read(&install.exe).expect("read"),
        std::fs::read(built()).expect("read"),
        "the old program is back at its own name"
    );
    assert!(!install.old.exists());
    assert!(
        install.cache("rolled-back").exists(),
        "the reason is kept for the user"
    );
}

#[test]
fn a_started_exe_that_reports_the_wrong_version_is_rolled_back() {
    let install = Install::new(false);
    let out = install.helper("99.0.0");
    assert!(!out.status.success(), "{out:?}");
    assert!(install.exe.is_file());
    assert!(install.cache("rolled-back").exists());
}

#[test]
fn with_nothing_staged_nothing_changes() {
    let install = Install::new(false);
    std::fs::remove_file(&install.new).expect("remove");
    let before = std::fs::read(&install.exe).expect("read");
    let out = install.helper(env!("CARGO_PKG_VERSION"));
    assert!(out.status.success(), "{out:?}");
    assert_eq!(std::fs::read(&install.exe).expect("read"), before);
}
