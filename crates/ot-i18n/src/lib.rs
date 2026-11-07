//! Fluent bundles, locale detection and the one date/number formatter.
//!
//! `format_datetime` is the only function in the workspace that formats a
//! date, a time or a number. The toast, the tray tooltip and the speech
//! service all call it, so a format change lands in one place.
//!
//! See `docs/03-architecture.md` for where this crate sits and which crates it
//! may depend on.
