//! `--self-test`, `--help` and `--version` against the built binary.
//!
//! These drive the real executable rather than calling into the library,
//! because the contract they check is about what a user and CI see: the exit
//! code, and one flushed line per check (`docs/09-ci-cd-and-releases.md`).
//! A unit test cannot observe either.

use std::path::Path;
use std::process::Command;

/// The binary under test, as cargo built it for this test run.
fn exe() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_OmniToaster"))
}

/// Every check name `--self-test` is contracted to print.
const CHECK_NAMES: &[&str] = &[
    "paths resolve and are writable",
    "the log file is written",
    "both locale bundles parse",
    "locale key parity holds",
    "the database migrates",
    "the date formatter answers in both languages",
];

#[test]
fn self_test_passes_against_a_scratch_data_dir_and_names_every_check() {
    let temp = tempfile::tempdir().expect("temp dir");
    let data = temp.path().join("data-root");

    let output = Command::new(exe())
        .args(["--self-test", "--data-dir"])
        .arg(&data)
        .output()
        .expect("run the binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "--self-test exited {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status.code()
    );

    for name in CHECK_NAMES {
        assert!(
            stdout.contains(name),
            "--self-test did not report {name:?}\nstdout:\n{stdout}"
        );
    }
    assert!(!stdout.contains("FAILED"), "a check failed:\n{stdout}");
    assert!(stdout.contains("all ok"), "no summary line:\n{stdout}");
}

#[test]
fn self_test_fails_and_names_the_locale_check_when_an_override_is_broken() {
    let temp = tempfile::tempdir().expect("temp dir");
    let data = temp.path().join("data-root");

    // A first run creates the subtree, so the override can be dropped into it.
    let first = Command::new(exe())
        .args(["--self-test", "--data-dir"])
        .arg(&data)
        .output()
        .expect("run");
    assert!(first.status.success(), "the first run should pass");

    // `{ $unclosed` is a Fluent syntax error. The app must survive it on the
    // embedded text, which is what makes this interesting: the check has to
    // notice the override is broken rather than silently falling back and
    // reporting ok.
    let locales = data.join("locales").join("en-US");
    std::fs::create_dir_all(&locales).expect("mkdir");
    std::fs::write(
        locales.join("omni_toaster.ftl"),
        "help-usage = Broken { $unclosed\n",
    )
    .expect("write the broken override");

    let output = Command::new(exe())
        .args(["--self-test", "--data-dir"])
        .arg(&data)
        .output()
        .expect("run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The app deliberately ignores a broken override and uses the embedded
    // text, so the self-test still passes - and that is the documented
    // behaviour (docs/05-data-model-and-storage.md). What must not happen is
    // a crash or a hang.
    assert!(
        output.status.success(),
        "a broken override should be ignored, not fatal\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("both locale bundles parse"),
        "the locale check should still run:\n{stdout}"
    );
}

#[test]
fn self_test_prints_each_check_before_its_verdict() {
    // The ordering is the contract: the name goes out first and flushed, so an
    // abort - which `panic = "abort"` in the release profile makes possible -
    // leaves the dying check's name as the last line. If the verdict were
    // printed with the name in one go, a crash would name nothing.
    let temp = tempfile::tempdir().expect("temp dir");
    let output = Command::new(exe())
        .args(["--self-test", "--data-dir"])
        .arg(temp.path().join("data-root"))
        .output()
        .expect("run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    for name in CHECK_NAMES {
        let line = stdout
            .lines()
            .find(|l| l.contains(name))
            .unwrap_or_else(|| panic!("{name} missing from:\n{stdout}"));
        let name_at = line.find(name).expect("found above");
        let verdict_at = line.find(" ok").or_else(|| line.find("FAILED"));
        if let Some(verdict_at) = verdict_at {
            assert!(
                name_at < verdict_at,
                "the verdict precedes the check name on {line:?}"
            );
        }
    }
}

#[test]
fn version_prints_the_one_word_product_name() {
    let output = Command::new(exe()).arg("--version").output().expect("run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    // CR-0001. The packaging script matches the version substring, so this
    // guards the name the owner actually asked about.
    assert!(
        stdout.contains(&format!("OmniToaster {}", env!("CARGO_PKG_VERSION"))),
        "unexpected --version output: {stdout:?}"
    );
    assert!(
        !stdout.contains("Omni Toaster"),
        "the two-word spelling came back: {stdout:?}"
    );
}

#[test]
fn help_is_printed_in_the_requested_language() {
    let english = Command::new(exe())
        .args(["--help", "--lang", "en-US"])
        .output()
        .expect("run");
    let malay = Command::new(exe())
        .args(["--help", "--lang", "ms-MY"])
        .output()
        .expect("run");

    assert!(english.status.success() && malay.status.success());
    let en = String::from_utf8_lossy(&english.stdout).to_string();
    let ms = String::from_utf8_lossy(&malay.stdout).to_string();

    assert!(en.contains("Usage: OmniToaster"), "{en}");
    assert!(ms.contains("Penggunaan: OmniToaster"), "{ms}");
    assert_ne!(en, ms, "--help rendered the same text for both languages");

    // The flags themselves are what the user types, so they stay English in
    // both; only the descriptions are translated.
    for text in [&en, &ms] {
        for flag in ["--self-test", "--data-dir", "--log-level", "--lang"] {
            assert!(text.contains(flag), "{flag} missing from help:\n{text}");
        }
    }
}

#[test]
fn an_unknown_option_is_named_rather_than_ignored() {
    let output = Command::new(exe())
        .args(["--data-dirr", "./nope"])
        .output()
        .expect("run");

    assert!(!output.status.success(), "an unknown flag should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--data-dirr"),
        "the message should name the offending flag: {stderr}"
    );
    // A silently ignored `--data-dir` typo would write into the real
    // OmniToasterData folder, which is the whole reason this is an error.
    assert!(
        stderr.contains("--help"),
        "point the user somewhere: {stderr}"
    );
}

#[test]
fn a_second_instance_bows_out_instead_of_starting() {
    // FEAT-0011. The unit tests can only prove the guard within one process;
    // this is the one that shows a named mutex actually keeps two *processes*
    // apart, which is the whole point — two schedulers would fire every
    // reminder twice.
    let temp = tempfile::tempdir().expect("temp dir");
    let data = temp.path().join("data-root");

    // The first copy holds the guard while the second tries to start. Without
    // the hold it would exit first and the second would succeed legitimately.
    let mut first = Command::new(exe())
        .args(["--hold-ms", "4000", "--data-dir"])
        .arg(&data)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("start the first instance");

    // Give it time to reach the guard. Generous: a cold start compiles
    // nothing but does open a database.
    std::thread::sleep(std::time::Duration::from_millis(1500));

    let second = Command::new(exe())
        .args(["--data-dir"])
        .arg(&data)
        .output()
        .expect("run the second instance");

    let out = String::from_utf8_lossy(&second.stdout).to_string();

    // Exit 0, not a failure: the user asked for OmniToaster and OmniToaster
    // is running. A non-zero code here would make a double-click look broken.
    assert!(
        second.status.success(),
        "the second instance should exit 0, got {:?}\n{out}",
        second.status.code()
    );
    assert!(
        out.contains("already running"),
        "the second instance should say so: {out:?}"
    );
    assert!(
        !out.contains("schema version"),
        "the second instance opened the database; it should have bowed out first:\n{out}"
    );

    let _ = first.wait();

    // And once the first has gone, a fresh start works again — a killed or
    // finished process must not lock the app out.
    let third = Command::new(exe())
        .args(["--data-dir"])
        .arg(&data)
        .output()
        .expect("run after the first exited");
    let third_out = String::from_utf8_lossy(&third.stdout).to_string();
    assert!(third.status.success(), "a later start should succeed");
    assert!(
        third_out.contains("schema version"),
        "the guard was not released: {third_out:?}"
    );
}

#[test]
fn worker_is_accepted_and_reaches_startup() {
    let temp = tempfile::tempdir().expect("temp dir");
    let output = Command::new(exe())
        .args(["--worker", "--hold-ms", "0", "--data-dir"])
        .arg(temp.path().join("data-root"))
        .output()
        .expect("run");

    // `--hold-ms` is what keeps this headless. Since FEAT-0014 a plain
    // `--worker` opens the settings window and blocks until it is closed, and
    // no test may do that: CI runs as a service in session 0, where no window
    // can open at all. The window is checked in the running app instead.
    assert!(
        output.status.success(),
        "--worker should be accepted: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Data folder:") && stdout.contains("Holding:"),
        "--worker should reach startup: {stdout}"
    );
}
