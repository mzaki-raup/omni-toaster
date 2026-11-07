//! Audio playback and speech synthesis.
//!
//! One output stream, so an announcement and an Azan clip cannot fight over
//! the device. Azan is never interrupted or mixed.
//!
//! See `docs/03-architecture.md` for where this crate sits and which crates it
//! may depend on.
