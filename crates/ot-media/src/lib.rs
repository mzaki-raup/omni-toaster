//! Audio playback and speech synthesis.
//!
//! One output stream, so an announcement and an Azan clip cannot fight over
//! the device. Azan is never interrupted or mixed.
//!
//! [`audio`] checks a clip the user supplies; [`player`] plays one on a thread
//! of its own. See ADR-022 for `rodio`, and `docs/03-architecture.md` for where
//! this crate sits and which crates it may depend on.

pub mod audio;
pub mod player;

pub use player::{Clip, Player};
