//! Switching the language of a running app (FEAT-0016).
//!
//! These live in their own integration test — their own **process** — because
//! they change a process-wide global. A unit test that asserts English and a
//! test that switches to Malay cannot share a process without one of them
//! failing on thread scheduling, which is exactly the flakiness that took a
//! release down earlier in this project (`docs/GOTCHAS.md`).
//!
//! For the same reason this is **one** test rather than several: `cargo test`
//! runs the tests in one file in parallel threads, so two switching tests
//! would race each other just as surely.

use std::time::{Duration, Instant};

use ot_i18n::Language;

/// A key whose Malay text is definitely different from its English text, so a
/// switch that did nothing cannot pass.
const KEY: &str = "settings-window-title";

#[test]
fn the_language_changes_while_the_app_runs() {
    ot_i18n::init(Language::English).expect("the first install succeeds");
    assert_eq!(ot_i18n::current_language(), Some(Language::English));
    let english = ot_i18n::text(KEY);
    assert_eq!(english, "Settings", "the bundled English text");

    // A second init is refused rather than quietly overwriting: two callers
    // initialising is a defect worth naming.
    assert!(
        matches!(
            ot_i18n::init(Language::Malay),
            Err(ot_i18n::I18nError::AlreadyInitialised)
        ),
        "init must not be the way to change the language"
    );
    assert_eq!(
        ot_i18n::current_language(),
        Some(Language::English),
        "the refused init must not have changed anything"
    );

    // The switch itself.
    let started = Instant::now();
    ot_i18n::set_language(Language::Malay).expect("the switch succeeds");
    let took = started.elapsed();

    assert_eq!(ot_i18n::current_language(), Some(Language::Malay));
    let malay = ot_i18n::text(KEY);
    assert_eq!(malay, "Tetapan", "the bundled Malay text");
    assert_ne!(english, malay, "the switch changed nothing");

    // Phase 1's exit criterion is that the whole switch — both surfaces —
    // completes in under a second. Rebuilding the catalogues is the slow part
    // of it, so it is held to a small fraction of that budget: if parsing both
    // files ever costs a tenth of a second, the switch is in trouble before
    // anything has been redrawn.
    assert!(
        took < Duration::from_millis(100),
        "rebuilding the catalogues took {took:?}, which leaves nothing for the redraw"
    );

    // Arguments are substituted against the language in force at the time,
    // not the one the key was first looked up in.
    let with_argument =
        ot_i18n::text_with("settings-status-paused-until", &[("time", "13:20".into())]);
    assert!(
        with_argument.contains("13:20"),
        "the argument is missing: {with_argument:?}"
    );
    assert!(
        !with_argument.contains('\u{2068}'),
        "Fluent's isolating marks reached the caller: {with_argument:?}"
    );
    assert_ne!(
        with_argument,
        ot_i18n::text("settings-status-paused-until"),
        "the argument was not substituted at all"
    );

    // And back again, including to the language it started in — a user who
    // switches away and back must not be left with half of each.
    ot_i18n::set_language(Language::English).expect("switching back succeeds");
    assert_eq!(ot_i18n::current_language(), Some(Language::English));
    assert_eq!(ot_i18n::text(KEY), english);

    // Switching to the language already in force is not a special case.
    ot_i18n::set_language(Language::English).expect("a no-op switch succeeds");
    assert_eq!(ot_i18n::text(KEY), english);
}
