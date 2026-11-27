//! Rebuild when the release repository changes.
//!
//! Which forge a copy updates from is fixed when it is built, by the release
//! script that builds it, through `OMNITOASTER_RELEASE_REPO` (ADR-024). `option_env!`
//! tracks that variable on its own in current Cargo, but a runner whose target
//! directory is shared between builds for two forges must never reuse a crate
//! compiled for the other one, so this says so outright.

fn main() {
    println!("cargo:rerun-if-env-changed=OMNITOASTER_RELEASE_REPO");
    println!("cargo:rerun-if-changed=build.rs");
}
