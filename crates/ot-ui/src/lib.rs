//! Shared egui widgets, the theme and the settings window.
//!
//! No library crate depends on this one. Hover changes a control's fill or
//! border; it never grows, moves or scales a glyph.
//!
//! See `docs/03-architecture.md` for where this crate sits and which crates it
//! may depend on.

pub mod layout;
pub mod settings_window;
pub mod theme;

pub use settings_window::{SettingsWindow, WindowError};
pub use theme::{Appearance, Theme, ThemeMode, Tokens};
