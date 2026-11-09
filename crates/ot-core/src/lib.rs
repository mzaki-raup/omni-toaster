//! Settings, portable paths, the event bus, errors, logging and the panic hook.
//!
//! Depends on nothing else in the workspace. Everything else depends on
//! this, so anything added here is added for every crate: keep it to the
//! pieces that genuinely are shared.
//!
//! See `docs/03-architecture.md` for where this crate sits and which crates it
//! may depend on.

pub mod logging;
pub mod paths;
pub mod pause;
pub mod settings;

pub use logging::{HostInfo, LogError, LogGuard, RecentLines};
pub use paths::{DataRoot, PathError};
pub use pause::{Pause, PauseFor};
pub use settings::{General, Settings, SettingsError};
