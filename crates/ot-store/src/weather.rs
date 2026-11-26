//! The last weather answer (FEAT-0067, `docs/specs/weather.md` §7 and §8).
//!
//! One row of `weather_cache`, for the one saved place, holding the response body
//! as the service sent it and when it was fetched. The store does not read the
//! payload — `ot-weather` does, with the same strict parser that read it the first
//! time — so a damaged row is a failed parse and a refetch, never a wrong
//! forecast. The cache is disposable: emptying it costs a fetch and nothing else.
//! It is emptied when the saved place moves, because another place's weather is
//! not this place's.

use jiff::{SignedDuration, Timestamp};
use rusqlite::{OptionalExtension as _, params};

use crate::location::ROW;
use crate::reminders::stamp;
use crate::{Store, StoreError};

/// How old cached weather may be before it counts as stale: two hours, four
/// polls at the default interval. A stale answer is still shown, marked as old,
/// and never produces a notification (`docs/specs/weather.md` §7).
pub const DEFAULT_TTL: SignedDuration = SignedDuration::from_hours(2);

/// The cached answer and when it was fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedWeather {
    pub fetched_at: Timestamp,
    /// The body exactly as Open-Meteo sent it.
    pub payload_json: String,
}

impl CachedWeather {
    /// How long ago it was fetched. Never negative: a clock that went back makes
    /// it new, not from the future.
    pub fn age(&self, now: Timestamp) -> SignedDuration {
        now.duration_since(self.fetched_at)
            .max(SignedDuration::ZERO)
    }

    /// Whether it is older than `ttl`. Exactly `ttl` old is still fresh.
    pub fn is_stale(&self, now: Timestamp, ttl: SignedDuration) -> bool {
        self.age(now) > ttl
    }
}

impl Store {
    /// Keep `payload_json`, fetched at `fetched_at`, as the cached weather of the
    /// saved place, replacing whatever was there. It needs a saved place: weather
    /// is for somewhere.
    pub fn write_weather(
        &self,
        fetched_at: Timestamp,
        payload_json: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO weather_cache (location_id, fetched_at, payload_json) \
             VALUES (?1, ?2, ?3) \
             ON CONFLICT (location_id) DO UPDATE SET fetched_at = excluded.fetched_at, \
             payload_json = excluded.payload_json",
            params![ROW, stamp(fetched_at), payload_json],
        )?;
        Ok(())
    }

    /// The cached weather, if there is any.
    pub fn weather(&self) -> Result<Option<CachedWeather>, StoreError> {
        let row: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT fetched_at, payload_json FROM weather_cache WHERE location_id = ?1",
                params![ROW],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((fetched_at, payload_json)) = row else {
            return Ok(None);
        };
        // A time this build cannot read is a cache it cannot use, which is the
        // same as having none.
        match fetched_at.parse::<Timestamp>() {
            Ok(fetched_at) => Ok(Some(CachedWeather {
                fetched_at,
                payload_json,
            })),
            Err(error) => {
                tracing::warn!(%error, "the cached weather has an unreadable time and was ignored");
                Ok(None)
            }
        }
    }

    /// Empty the cache.
    pub fn clear_weather(&self) -> Result<(), StoreError> {
        self.conn.execute("DELETE FROM weather_cache", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::{Origin, StoredLocation};

    fn at(text: &str) -> Timestamp {
        text.parse().expect("a time")
    }

    fn place(lat: f64, lon: f64) -> StoredLocation {
        StoredLocation {
            name: "Somewhere".into(),
            latitude: lat,
            longitude: lon,
            tz: "Asia/Kuala_Lumpur".into(),
            country_code: Some("MY".into()),
            origin: Origin::Manual,
            updated_at: at("2026-11-26T00:00:00Z"),
        }
    }

    fn store_with_place() -> Store {
        let store = Store::open_in_memory().expect("opens");
        store
            .save_manual_location(&place(3.15, 101.69))
            .expect("saved");
        store
    }

    #[test]
    fn what_is_written_reads_back_and_a_new_answer_replaces_it() {
        let store = store_with_place();
        assert_eq!(store.weather().expect("reads"), None, "nothing yet");
        store
            .write_weather(at("2026-11-26T11:00:00Z"), r#"{"a":1}"#)
            .expect("written");
        assert_eq!(
            store.weather().expect("reads"),
            Some(CachedWeather {
                fetched_at: at("2026-11-26T11:00:00Z"),
                payload_json: r#"{"a":1}"#.into()
            })
        );
        store
            .write_weather(at("2026-11-26T11:30:00Z"), r#"{"b":2}"#)
            .expect("written");
        let now = store.weather().expect("reads").expect("a row");
        assert_eq!(now.payload_json, r#"{"b":2}"#, "one row, replaced");
        let rows: i64 = store
            .conn
            .query_row("SELECT count(*) FROM weather_cache", [], |r| r.get(0))
            .expect("counts");
        assert_eq!(rows, 1);
    }

    #[test]
    fn age_and_staleness_follow_the_ttl_exactly() {
        let cached = CachedWeather {
            fetched_at: at("2026-11-26T10:00:00Z"),
            payload_json: String::new(),
        };
        assert_eq!(
            cached.age(at("2026-11-26T10:30:00Z")),
            SignedDuration::from_mins(30)
        );
        assert!(!cached.is_stale(at("2026-11-26T11:59:59Z"), DEFAULT_TTL));
        assert!(
            !cached.is_stale(at("2026-11-26T12:00:00Z"), DEFAULT_TTL),
            "exactly the TTL is fresh"
        );
        assert!(cached.is_stale(at("2026-11-26T12:00:01Z"), DEFAULT_TTL));
        // A clock that went back is not an age from the future.
        assert_eq!(cached.age(at("2026-11-26T09:00:00Z")), SignedDuration::ZERO);
        assert!(!cached.is_stale(at("2026-11-26T09:00:00Z"), DEFAULT_TTL));
    }

    #[test]
    fn it_needs_a_saved_place() {
        let store = Store::open_in_memory().expect("opens");
        assert!(
            store
                .write_weather(at("2026-11-26T11:00:00Z"), "{}")
                .is_err()
        );
    }

    #[test]
    fn emptying_it_costs_nothing_but_a_fetch() {
        let store = store_with_place();
        store
            .write_weather(at("2026-11-26T11:00:00Z"), "{}")
            .expect("written");
        store.clear_weather().expect("cleared");
        assert_eq!(store.weather().expect("reads"), None);
    }

    #[test]
    fn moving_the_place_empties_it_and_saving_the_same_place_again_does_not() {
        let store = store_with_place();
        store
            .write_weather(at("2026-11-26T11:00:00Z"), "{}")
            .expect("written");
        // The daily lookup writing the same place again keeps the weather.
        store
            .save_manual_location(&place(3.15, 101.69))
            .expect("saved");
        assert!(store.weather().expect("reads").is_some());
        // Another place does not.
        store
            .save_manual_location(&place(1.55, 110.35))
            .expect("saved");
        assert_eq!(store.weather().expect("reads"), None);
    }

    #[test]
    fn a_damaged_time_is_the_same_as_no_cache() {
        let store = store_with_place();
        store
            .write_weather(at("2026-11-26T11:00:00Z"), "{}")
            .expect("written");
        store
            .conn
            .execute("UPDATE weather_cache SET fetched_at = 'yesterday'", [])
            .expect("damaged");
        assert_eq!(store.weather().expect("reads"), None);
    }

    #[test]
    fn it_survives_a_restart_and_lands_in_the_data_folder() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        std::fs::create_dir_all(dir.path().join("data")).expect("data folder");
        {
            let store = Store::open(dir.path()).expect("opens");
            store
                .save_manual_location(&place(3.15, 101.69))
                .expect("saved");
            store
                .write_weather(at("2026-11-26T11:00:00Z"), r#"{"kept":true}"#)
                .expect("written");
        }
        let again = Store::open(dir.path()).expect("opens again");
        assert_eq!(
            again.weather().expect("reads").map(|c| c.payload_json),
            Some(r#"{"kept":true}"#.to_owned())
        );
        let database = dir.path().join(crate::DB_RELATIVE_PATH);
        assert!(
            database.is_file(),
            "the cache is in the one database under the data folder"
        );
        // Nothing else was written beside it.
        let others: Vec<_> = std::fs::read_dir(dir.path().join("data"))
            .expect("lists")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .filter(|name| !name.starts_with("omnitoaster.db"))
            .collect();
        assert!(others.is_empty(), "{others:?}");
    }
}
