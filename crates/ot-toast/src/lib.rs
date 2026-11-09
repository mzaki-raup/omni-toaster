//! `ToastSpec`, the toast renderer and the animations.
//!
//! The renderer draws only a `ToastSpec`, which is why the Settings preview
//! and a real toast cannot diverge. Nothing inside the frame loop parses,
//! lays out or decodes without caching on a hash of its inputs.
//!
//! See `docs/03-architecture.md` for where this crate sits and which crates it
//! may depend on.

pub mod content;
pub mod placement;
pub mod prefs;
pub mod render;
pub mod spec;
pub mod viewport;

pub use placement::{Position, StackDirection};
pub use prefs::{Header, Prefs};
pub use spec::{Behaviour, Content, Rgba, Shadow, Shape, Style, ToastSpec};
pub use viewport::{Disposition, LiveToast, ToastId, Toasts};
