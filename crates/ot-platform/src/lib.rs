//! Every Windows API call, behind interfaces named for the capability.
//!
//! The ONLY crate allowed to contain platform code: tray, autostart,
//! system_theme, single_instance, window_effects, supervisor, speech.
//! Another crate reaching for `windows` or `winreg` directly is a defect -
//! it is what would block the eventual macOS and Linux work. Windows paths
//! are `#[cfg(windows)]` so clippy passes on Linux.
//!
//! See `docs/03-architecture.md` for where this crate sits and which crates it
//! may depend on.

pub mod autostart;
pub mod locale;
pub mod monitors;
pub mod single_instance;
pub mod system_theme;
pub mod toast_window;
pub mod tray;
pub mod ui_font;

pub use autostart::{Autostart, AutostartError};
pub use locale::user_locale;
pub use monitors::{Monitor, monitors};
pub use single_instance::{InstanceError, InstanceGuard, Role};
pub use system_theme::{SystemTheme, system_theme};
pub use tray::{Labels, PauseWindow, Tray, TrayCommand, TrayError};
pub use ui_font::{Font, FontError};
