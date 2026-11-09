//! The workspace's own manifests, checked for the mistakes that only surface
//! during a release.
//!
//! These are not unit tests of any crate: they are the checks that would
//! otherwise be made by a release going red. A release moves
//! `[workspace.package] version` and then builds, so a manifest that disagrees
//! with that single version only fails at the worst moment.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/omnitoaster.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the binary crate is two levels below the workspace root")
        .to_path_buf()
}

fn manifests() -> Vec<PathBuf> {
    let root = workspace_root();
    let mut found = vec![root.join("Cargo.toml")];
    let crates = fs::read_dir(root.join("crates")).expect("crates/ is readable");
    for entry in crates {
        let path = entry.expect("a readable directory entry").path();
        let manifest = path.join("Cargo.toml");
        if manifest.is_file() {
            found.push(manifest);
        }
    }
    assert!(found.len() > 2, "no crate manifests found: {found:?}");
    found
}

/// A `path` dependency on a sibling crate must not carry a `version`.
///
/// The version lives once, in `[workspace.package] version` (ADR-001), and
/// `scripts/update-version.ps1` moves it. A sibling pinned as
/// `{ version = "0.4.0", path = "../ot-platform" }` keeps that literal while
/// the crate it points at becomes 0.5.0, and cargo then refuses to resolve:
/// "failed to select a version for the requirement `ot-platform = ^0.4.0`".
///
/// A caret requirement hides this through every patch release — `^0.4.0`
/// accepts 0.4.1 — so the first minor bump is what breaks, which is exactly
/// the release nobody is watching for a resolver error (BUG-0003).
#[test]
fn no_path_dependency_pins_a_version() {
    let mut offenders = Vec::new();
    for manifest in manifests() {
        let text = fs::read_to_string(&manifest).expect("a readable manifest");
        for (number, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            if line.contains("path = \"") && line.contains("version = \"") {
                offenders.push(format!("{}:{}: {line}", manifest.display(), number + 1));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a path dependency carries its own version, which a release bump will \
         leave behind (ADR-001):\n{}",
        offenders.join("\n")
    );
}
