//! `PrayerSource`, the JAKIM and calculated sources, and the zone table.
//!
//! The scheduler sees a list of times plus a source label and nothing else,
//! so the fallback chain lives in exactly one place.
//!
//! See `docs/03-architecture.md` for where this crate sits and which crates it
//! may depend on.
