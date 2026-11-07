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
//! Phase 0 implements none of those roles. Argument parsing arrives with
//! FEAT-0006 and the supervisor with Phase 8; for now the binary reports its
//! version, which is what `scripts/build-and-package.ps1` checks before it
//! packages a release.

/// Printed by `--version` and compared against the tag being released, so a
/// binary built from the wrong `Cargo.toml` cannot be packaged.
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    println!("Omni Toaster {VERSION}");
}

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn version_is_populated_from_the_workspace_manifest() {
        // The release pipeline matches this value against the tag it is
        // publishing; an empty or placeholder version would let every release
        // offer itself as an update for ever.
        assert!(!VERSION.is_empty(), "CARGO_PKG_VERSION is empty");
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(
            parts.len(),
            3,
            "expected a three-part version, got {VERSION}"
        );
        for part in parts {
            let numeric = part.split('-').next().unwrap_or_default();
            assert!(
                !numeric.is_empty() && numeric.chars().all(|c| c.is_ascii_digit()),
                "version component {part:?} in {VERSION} is not numeric"
            );
        }
    }
}
