//! Shared egui widgets, the theme and the settings window.
//!
//! No library crate depends on this one. Hover changes a control's fill or
//! border; it never grows, moves or scales a glyph.
//!
//! See `docs/03-architecture.md` for where this crate sits and which crates it
//! may depend on.

pub mod animation;
pub mod appearance;
pub mod fields;
pub mod images;
pub mod layout;
pub mod library;
pub mod phrases;
pub mod prayer;
pub mod prayer_pane;
pub mod prayer_service;
pub mod reminder_book;
pub mod reminder_editor;
pub mod reminder_toast;
pub mod reminders;
pub mod settings_window;
pub mod theme;
pub mod weather_pane;
pub mod weather_service;

pub use settings_window::{SettingsWindow, WindowError};
pub use theme::{Appearance, Theme, ThemeMode, Tokens};
