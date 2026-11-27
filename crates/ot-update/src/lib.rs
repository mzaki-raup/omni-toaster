//! Where this copy looks for newer releases, and the check itself
//! (`docs/specs/updater.md`, ADR-024).
//!
//! Step one of the in-app updater (FEAT-0077): find out whether a newer release
//! exists on the forge this copy was built for, GitHub or git.zacx.net. Verifying,
//! downloading and installing it are the steps that follow (FEAT-0078 onward).

pub mod check;
pub mod download;
pub mod manifest;
pub mod release;
pub mod source;
pub mod stage;
pub mod verify;

#[cfg(test)]
mod chain;

pub use ot_net::Cancel;
