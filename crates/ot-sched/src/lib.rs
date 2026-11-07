//! The scheduler thread, recurrence and snooze.
//!
//! The single source of time for the whole app. It stores each reminder's
//! next absolute fire time, recomputes after a DST change, a resume or a
//! clock change, and never relies on a long sleep.
//!
//! See `docs/03-architecture.md` for where this crate sits and which crates it
//! may depend on.
