//! SQLite, the numbered migrations and the content-addressed asset store.
//!
//! Every statement is parameterized; no SQL is built by string
//! concatenation. A shipped migration is never edited - a correction is a
//! new migration.
//!
//! See `docs/03-architecture.md` for where this crate sits and which crates it
//! may depend on.
