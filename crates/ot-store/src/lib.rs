//! SQLite, the numbered migrations and the content-addressed asset store.
//!
//! Every statement is parameterized; no SQL is built by string concatenation
//! (`docs/06-security-and-privacy.md`). A shipped migration is never edited — a
//! correction is a new migration, because an edit changes the schema of
//! databases that have already applied it and the runner cannot notice.
//!
//! See `docs/05-data-model-and-storage.md` for the schema and
//! `docs/03-architecture.md` for where this crate sits.

pub mod assets;
pub mod builtin;
pub mod library;
pub mod prayer;
pub mod reminders;

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension as _, params};

/// The migrations, in order, embedded so a portable exe carries its own schema.
///
/// The number is the `PRAGMA user_version` the migration takes the database to.
/// Adding one here is the whole of registering it.
const MIGRATIONS: &[(i64, &str, &str)] = &[
    (
        1,
        "001_init.sql",
        include_str!("../migrations/001_init.sql"),
    ),
    (
        2,
        "002_asset_label.sql",
        include_str!("../migrations/002_asset_label.sql"),
    ),
    (
        3,
        "003_locale_keys.sql",
        include_str!("../migrations/003_locale_keys.sql"),
    ),
    (
        4,
        "004_jakim.sql",
        include_str!("../migrations/004_jakim.sql"),
    ),
];

/// The schema version this build migrates to: the last migration's number.
pub const SCHEMA_VERSION: i64 = 4;

/// The database file, relative to the data root.
const DB_RELATIVE_PATH: &str = "data/omnitoaster.db";

/// Why the store could not be opened or migrated.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("the database {} could not be opened: {source}", path.display())]
    Open {
        path: PathBuf,
        source: rusqlite::Error,
    },

    #[error("migration {number} ({name}) failed: {source}")]
    Migration {
        number: i64,
        name: &'static str,
        source: rusqlite::Error,
    },

    /// The database is newer than this build understands. Carrying on would
    /// risk writing data an older schema cannot represent, so the caller is
    /// expected to tell the user to update rather than to continue.
    #[error(
        "the database is at schema version {found} but this build only knows {known}; \
         update OmniToaster"
    )]
    FromTheFuture { found: i64, known: i64 },

    #[error("a query failed: {0}")]
    Query(#[from] rusqlite::Error),
}

/// An open database with its migrations applied.
///
/// `Debug` prints the schema version and nothing else: a `Connection` has no
/// useful debug form, and the contents are the user's reminders, which should
/// not reach a log line or a crash report by accident
/// (`docs/06-security-and-privacy.md`).
pub struct Store {
    conn: Connection,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store")
            .field("schema_version", &self.schema_version().ok())
            .finish_non_exhaustive()
    }
}

impl Store {
    /// Open `<root>/data/omnitoaster.db`, applying any outstanding migrations.
    ///
    /// `root` is the resolved data root, so the caller has already proved the
    /// folder is writable (`ot_core::paths`).
    pub fn open(root: &Path) -> Result<Self, StoreError> {
        let path = root.join(DB_RELATIVE_PATH);
        let conn = Connection::open(&path).map_err(|source| StoreError::Open {
            path: path.clone(),
            source,
        })?;
        Self::from_connection(conn)
    }

    /// Open an in-memory database, for tests.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory().map_err(|source| StoreError::Open {
            path: PathBuf::from(":memory:"),
            source,
        })?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self, StoreError> {
        // WAL survives a crash better and lets the scheduler read while the UI
        // writes. Foreign keys are off by default in SQLite, which would make
        // every ON DELETE clause in the schema decorative.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        // A crash mid-write should cost the last transaction, not the file.
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        let mut store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Apply every migration the database has not seen, each in its own
    /// transaction, and move `user_version` with it.
    fn migrate(&mut self) -> Result<(), StoreError> {
        let current: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;

        let known = MIGRATIONS.last().map_or(0, |(number, _, _)| *number);
        if current > known {
            return Err(StoreError::FromTheFuture {
                found: current,
                known,
            });
        }

        for (number, name, sql) in MIGRATIONS {
            if *number <= current {
                continue;
            }

            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)
                .map_err(|source| StoreError::Migration {
                    number: *number,
                    name,
                    source,
                })?;
            // `pragma_update` cannot be parameterized — SQLite does not accept
            // a binding for a pragma value — so the number is formatted in.
            // It comes from this file's own constant table, never from input.
            tx.pragma_update(None, "user_version", *number)?;
            tx.commit()?;

            tracing::info!(migration = number, name, "applied migration");
        }

        Ok(())
    }

    /// The schema version the database is at.
    pub fn schema_version(&self) -> Result<i64, StoreError> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?)
    }

    /// Read a settings value.
    ///
    /// `settings.toml` is the source of truth for configuration; this table
    /// mirrors only what the scheduler must read inside a transaction
    /// (ADR-007).
    pub fn setting(&self, key: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Write a settings value, replacing any previous one.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// The underlying connection, for the crates that own their own tables.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_schema_version_is_the_last_migrations_number() {
        assert_eq!(MIGRATIONS.last().map(|m| m.0), Some(SCHEMA_VERSION));
    }

    #[test]
    fn a_fresh_database_migrates_to_the_latest_version() {
        let store = Store::open_in_memory().expect("opens");
        assert_eq!(store.schema_version().expect("version"), 4);
    }

    #[test]
    fn every_table_the_spec_names_exists() {
        let store = Store::open_in_memory().expect("opens");
        let mut stmt = store
            .connection()
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .expect("prepare");
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .expect("query")
            .filter_map(Result::ok)
            .collect();

        for expected in [
            "asset",
            "category",
            "location",
            "notification_log",
            "reminder",
            "settings",
            "snooze_log",
            "weather_cache",
            "jakim_prayer_cache",
        ] {
            assert!(
                tables.iter().any(|t| t == expected),
                "{expected} is missing; tables are {tables:?}"
            );
        }
    }

    #[test]
    fn migrating_twice_is_a_no_op() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("data")).expect("mkdir");

        {
            let store = Store::open(dir.path()).expect("first open");
            store.set_setting("theme", "dark").expect("write");
            assert_eq!(store.schema_version().expect("version"), 4);
        }

        // Re-opening must not re-run 001 — which would fail on the existing
        // tables — and must leave the data alone.
        let store = Store::open(dir.path()).expect("second open");
        assert_eq!(store.schema_version().expect("version"), 4);
        assert_eq!(
            store.setting("theme").expect("read").as_deref(),
            Some("dark"),
            "re-opening must not touch existing data"
        );
    }

    #[test]
    fn a_version_1_database_gains_the_label_and_keeps_its_rows() {
        // The existing-database half of the migration test: a file built by
        // 001 alone, with an asset in it, opened by this build.
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("data")).expect("mkdir");
        {
            let conn =
                Connection::open(dir.path().join(DB_RELATIVE_PATH)).expect("a version 1 file");
            conn.execute_batch(MIGRATIONS[0].2).expect("001");
            conn.pragma_update(None, "user_version", 1i64)
                .expect("version 1");
            conn.execute(
                "INSERT INTO asset (sha256, kind, relative_path, bytes, created_at) \
                 VALUES ('abc', 'image', 'assets/abc.png', 3, '2026-11-10')",
                [],
            )
            .expect("an old row");
        }

        let store = Store::open(dir.path()).expect("migrates");
        assert_eq!(store.schema_version().expect("version"), 4);
        let label: String = store
            .connection()
            .query_row("SELECT label FROM asset WHERE sha256 = 'abc'", [], |row| {
                row.get(0)
            })
            .expect("the old row is still there");
        assert_eq!(label, "", "an old row gets an empty label, not a failure");
    }

    #[test]
    fn a_version_2_database_gains_the_keys_and_keeps_its_rows() {
        // 003 on an existing database: a file built by 001 and 002, as
        // v0.11.0 left it, with a template and a reminder in it.
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("data")).expect("mkdir");
        {
            let conn =
                Connection::open(dir.path().join(DB_RELATIVE_PATH)).expect("a version 2 file");
            conn.execute_batch(MIGRATIONS[0].2).expect("001");
            conn.execute_batch(MIGRATIONS[1].2).expect("002");
            conn.pragma_update(None, "user_version", 2i64)
                .expect("version 2");
            conn.execute(
                "INSERT INTO category (id, kind, custom_title) VALUES (1, 'prayer', 'Subuh')",
                [],
            )
            .expect("an old template");
            conn.execute(
                "INSERT INTO reminder (id, title, trigger_type, trigger_json, created_at, updated_at) \
                 VALUES (1, 'Old', 'at', '{}', '2026-11-10', '2026-11-10')",
                [],
            )
            .expect("an old reminder");
        }

        let store = Store::open(dir.path()).expect("migrates");
        assert_eq!(store.schema_version().expect("version"), 4);
        let conn = store.connection();

        let (title, key, lang): (String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT custom_title, prayer_key, authored_lang FROM category WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("the old template is still there");
        assert_eq!((title.as_str(), key, lang), ("Subuh", None, None));

        let lang: Option<String> = conn
            .query_row(
                "SELECT authored_lang FROM reminder WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .expect("the old reminder is still there");
        assert_eq!(lang, None);

        // The built-in types arrived next to it, not over it.
        let types: i64 = conn
            .query_row(
                "SELECT count(*) FROM category WHERE kind = 'reminder' AND type_key IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(types, 12);
    }

    #[test]
    fn a_version_3_database_gains_the_prayer_cache_and_keeps_its_rows() {
        // 004 on an existing database: a file built by 001 to 003, as v0.12.0
        // left it, with a reminder in it.
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("data")).expect("mkdir");
        {
            let conn =
                Connection::open(dir.path().join(DB_RELATIVE_PATH)).expect("a version 3 file");
            for migration in &MIGRATIONS[..3] {
                conn.execute_batch(migration.2)
                    .expect("an earlier migration");
            }
            conn.pragma_update(None, "user_version", 3i64)
                .expect("version 3");
            conn.execute(
                "INSERT INTO reminder (id, title, trigger_type, trigger_json, created_at, updated_at) \
                 VALUES (1, 'Old', 'at', '{}', '2026-11-10', '2026-11-10')",
                [],
            )
            .expect("an old reminder");
        }

        let store = Store::open(dir.path()).expect("migrates");
        assert_eq!(store.schema_version().expect("version"), 4);
        let conn = store.connection();
        let title: String = conn
            .query_row("SELECT title FROM reminder WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("the old reminder is still there");
        assert_eq!(title, "Old");
        let cached: i64 = conn
            .query_row("SELECT count(*) FROM jakim_prayer_cache", [], |row| {
                row.get(0)
            })
            .expect("the cache table exists and is empty");
        assert_eq!(cached, 0);
    }

    #[test]
    fn a_fresh_database_has_every_built_in_reminder_type_once() {
        let store = Store::open_in_memory().expect("opens");
        let mut stmt = store
            .connection()
            .prepare(
                "SELECT type_key FROM category WHERE kind = 'reminder' \
                 AND custom_title IS NULL AND custom_body IS NULL ORDER BY id",
            )
            .expect("prepare");
        let keys: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("rows");
        assert_eq!(
            keys,
            [
                "reminder-event",
                "reminder-task",
                "reminder-anniversary",
                "reminder-fatigue-eye",
                "reminder-fatigue-posture",
                "reminder-fatigue-hydration",
                "reminder-fatigue-work-hours",
                "reminder-timer",
                "reminder-alarm",
                "reminder-payment",
                "reminder-appointment",
                "reminder-custom",
            ]
        );

        // A key is one template: a second row with it is refused.
        let duplicate = store.connection().execute(
            "INSERT INTO category (kind, type_key) VALUES ('reminder', 'reminder-alarm')",
            [],
        );
        assert!(duplicate.is_err(), "a second reminder-alarm template");
    }

    #[test]
    fn a_settings_value_round_trips_and_overwrites() {
        let store = Store::open_in_memory().expect("opens");

        assert_eq!(store.setting("language").expect("read"), None);

        store.set_setting("language", "ms-MY").expect("write");
        assert_eq!(
            store.setting("language").expect("read").as_deref(),
            Some("ms-MY")
        );

        store.set_setting("language", "en-US").expect("overwrite");
        assert_eq!(
            store.setting("language").expect("read").as_deref(),
            Some("en-US"),
            "a second write should replace, not duplicate or fail"
        );
    }

    #[test]
    fn a_database_from_the_future_is_refused_rather_than_used() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("data")).expect("mkdir");
        let path = dir.path().join(DB_RELATIVE_PATH);

        {
            let conn = Connection::open(&path).expect("open");
            conn.pragma_update(None, "user_version", 99i64)
                .expect("set version");
        }

        let error = Store::open(dir.path()).expect_err("must refuse");
        assert!(
            matches!(
                error,
                StoreError::FromTheFuture {
                    found: 99,
                    known: 4
                }
            ),
            "got {error:?}"
        );
        // The message has to tell the user what to do about it.
        assert!(error.to_string().contains("update"), "{error}");
    }

    #[test]
    fn foreign_keys_are_enforced() {
        // Without `PRAGMA foreign_keys = ON` every ON DELETE clause in the
        // schema is decorative, and SQLite defaults it off.
        let store = Store::open_in_memory().expect("opens");
        let result = store.connection().execute(
            "INSERT INTO snooze_log (reminder_id, snoozed_until, count) VALUES (?1, ?2, ?3)",
            params![424242, "2026-11-08T13:00:00+08:00", 1],
        );
        assert!(
            result.is_err(),
            "a snooze for a reminder that does not exist should be refused"
        );
    }

    #[test]
    fn deleting_a_reminder_cascades_to_its_snoozes() {
        let store = Store::open_in_memory().expect("opens");
        let conn = store.connection();

        conn.execute(
            "INSERT INTO reminder (id, title, trigger_type, trigger_json, created_at, updated_at) \
             VALUES (1, ?1, 'at', '{}', ?2, ?2)",
            params!["Pay the bill", "2026-11-08T13:00:00+08:00"],
        )
        .expect("insert reminder");
        conn.execute(
            "INSERT INTO snooze_log (reminder_id, snoozed_until, count) VALUES (1, ?1, 1)",
            params!["2026-11-08T14:00:00+08:00"],
        )
        .expect("insert snooze");

        conn.execute("DELETE FROM reminder WHERE id = 1", [])
            .expect("delete");

        let remaining: i64 = conn
            .query_row("SELECT count(*) FROM snooze_log", [], |row| row.get(0))
            .expect("count");
        assert_eq!(remaining, 0, "the snooze row should have gone with it");
    }

    #[test]
    fn strict_tables_reject_a_wrong_type() {
        // The schema declares STRICT, which is the difference between a typo
        // being caught and a text value living in an INTEGER column for ever.
        let store = Store::open_in_memory().expect("opens");
        let result = store.connection().execute(
            "INSERT INTO asset (sha256, kind, relative_path, bytes, created_at) \
             VALUES ('abc', 'image', 'assets/images/abc.png', 'not a number', ?1)",
            params!["2026-11-08T13:00:00+08:00"],
        );
        assert!(
            result.is_err(),
            "STRICT should reject a text value in bytes"
        );
    }
}
