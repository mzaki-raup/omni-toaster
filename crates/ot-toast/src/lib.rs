//! `ToastSpec`, the toast renderer and the animations.
//!
//! The renderer draws only a `ToastSpec`, which is why the Settings preview
//! and a real toast cannot diverge. Nothing inside the frame loop parses,
//! lays out or decodes without caching on a hash of its inputs.
//!
//! See `docs/03-architecture.md` for where this crate sits and which crates it
//! may depend on.
