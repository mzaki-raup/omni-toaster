//! Rebuild when the release repository changes.
//!
//! Which forge a copy updates from is fixed when it is built, by the release
//! script that builds it, through `OMNITOASTER_RELEASE_REPO` (ADR-024). `option_env!`
//! tracks that variable on its own in current Cargo, but a runner whose target
//! directory is shared between builds for two forges must never reuse a crate
//! compiled for the other one, so this says so outright.
//!
//! It also makes the **source marker** the release check looks for in the finished
//! exe (FEAT-0081): `[omnitoaster-update-source=<repository>]`. The repository
//! alone would not do, because the default one is in every build as a constant, so
//! a GitHub exe contains the Forgejo's address too; the marker names only the one
//! the build is for. The default here must match `source::DEFAULT_REPO`, and a test
//! in that module says so.

const DEFAULT_REPO: &str = "https://git.zacx.net/owner/omni-toaster";

fn main() {
    println!("cargo:rerun-if-env-changed=OMNITOASTER_RELEASE_REPO");
    let repo =
        std::env::var("OMNITOASTER_RELEASE_REPO").unwrap_or_else(|_| DEFAULT_REPO.to_owned());
    println!("cargo:rustc-env=OMNITOASTER_UPDATE_SOURCE_MARKER=[omnitoaster-update-source={repo}]");
    println!("cargo:rerun-if-changed=build.rs");
}
