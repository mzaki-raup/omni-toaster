//! The last weather answer (FEAT-0067, `docs/specs/weather.md` §7 and §8).
//!
//! One row of `weather_cache`, for the weather place (`location` row 2, FEAT-0072), holding the response body
//! as the service sent it and when it was fetched. The store does not read the
//! payload — `ot-weather` does, with the same strict parser that read it the first
//! time — so a damaged row is a failed parse and a refetch, never a wrong
//! forecast. The cache is disposable: emptying it costs a fetch and nothing else.
//! It is emptied when the weather place moves, because another place's weather is
//! not this place's; moving the prayer place does not touch it.

use jiff::{SignedDuration, Timestamp};
use rusqlite::{OptionalExtension as _, params};

use crate::location::WEATHER_ROW;
use crate::reminders::stamp;
use crate::{Store, StoreError};

/// Where the class the change detector last told the user about is kept. It goes
/// with the cache when the weather place moves, so a new place starts from a fresh
/// baseline.
pub const NOTIFIED_SETTING: &str = "weather.notified";

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
    /// weather place, replacing whatever was there — **but only if the weather place
    /// is still at `latitude`, `longitude`**. A fetch that was under way when the user
    /// chose another place finishes with the old place's sky, and writing it would
    /// hand the new place a cache that looks fresh. Returns whether it was written.
    /// It needs a weather place: weather is for somewhere.
    pub fn write_weather(
        &self,
        latitude: f64,
        longitude: f64,
        fetched_at: Timestamp,
        payload_json: &str,
    ) -> Result<bool, StoreError> {
        let written = self.conn.execute(
            "INSERT INTO weather_cache (location_id, fetched_at, payload_json)              SELECT ?1, ?2, ?3 FROM location WHERE id = ?1 AND lat = ?4 AND lon = ?5              ON CONFLICT (location_id) DO UPDATE SET fetched_at = excluded.fetched_at,              payload_json = excluded.payload_json",
            params![WEATHER_ROW, stamp(fetched_at), payload_json, latitude, longitude],
        )?;
        Ok(written > 0)
    }

    /// The cached weather, if there is any.
    pub fn weather(&self) -> Result<Option<CachedWeather>, StoreError> {
        let row: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT fetched_at, payload_json FROM weather_cache WHERE location_id = ?1",
                params![WEATHER_ROW],
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
            .write_weather(3.15, 101.69, at("2026-11-26T11:00:00Z"), r#"{"a":1}"#)
            .expect("written");
        assert_eq!(
            store.weather().expect("reads"),
            Some(CachedWeather {
                fetched_at: at("2026-11-26T11:00:00Z"),
                payload_json: r#"{"a":1}"#.into()
            })
        );
        store
            .write_weather(3.15, 101.69, at("2026-11-26T11:30:00Z"), r#"{"b":2}"#)
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
    fn it_needs_a_weather_place_and_without_one_writes_nothing() {
        let store = Store::open_in_memory().expect("opens");
        assert!(
            !store
                .write_weather(3.15, 101.69, at("2026-11-26T11:00:00Z"), "{}")
                .expect("not an error")
        );
        assert_eq!(store.weather().expect("reads"), None);
    }

    #[test]
    fn emptying_it_costs_nothing_but_a_fetch() {
        let store = store_with_place();
        store
            .write_weather(3.15, 101.69, at("2026-11-26T11:00:00Z"), "{}")
            .expect("written");
        store.clear_weather().expect("cleared");
        assert_eq!(store.weather().expect("reads"), None);
    }

    #[test]
    fn moving_the_weather_place_empties_the_cache_and_the_told_class_and_the_same_place_does_not() {
        let store = store_with_place();
        store
            .write_weather(3.15, 101.69, at("2026-11-26T11:00:00Z"), "{}")
            .expect("written");
        store
            .set_setting(NOTIFIED_SETTING, "rain")
            .expect("a class was told");
        // Choosing the same place again keeps the weather and what was told.
        store
            .save_weather_location(&place(3.15, 101.69))
            .expect("saved");
        assert!(store.weather().expect("reads").is_some());
        assert_eq!(
            store.setting(NOTIFIED_SETTING).expect("reads").as_deref(),
            Some("rain")
        );
        // Another place does not: its sky is not this one's, and what was told about
        // this one's must not be compared with it.
        store
            .save_weather_location(&place(1.55, 110.35))
            .expect("saved");
        assert_eq!(store.weather().expect("reads"), None);
        assert_eq!(
            store.setting(NOTIFIED_SETTING).expect("reads").as_deref(),
            Some(""),
            "a fresh baseline"
        );
    }

    #[test]
    fn moving_the_prayer_place_leaves_the_weather_place_and_its_cache_alone() {
        let store = store_with_place();
        store
            .write_weather(3.15, 101.69, at("2026-11-26T11:00:00Z"), "{}")
            .expect("written");
        store
            .save_manual_location(&place(1.55, 110.35))
            .expect("another prayer place");
        assert!(store.weather().expect("reads").is_some(), "the cache stays");
        let weather = store.weather_location().expect("reads").expect("a place");
        assert_eq!((weather.latitude, weather.longitude), (3.15, 101.69));
        assert_eq!(
            store.location().expect("reads").expect("a place").latitude,
            1.55
        );
    }

    #[test]
    fn a_weather_place_is_saved_and_read_back_apart_from_the_prayer_place() {
        let store = store_with_place();
        let mut kuching = place(1.55, 110.35);
        kuching.name = "Kuching".into();
        store.save_weather_location(&kuching).expect("saved");
        assert_eq!(
            store
                .weather_location()
                .expect("reads")
                .expect("a place")
                .name,
            "Kuching"
        );
        assert_eq!(
            store.location().expect("reads").expect("a place").name,
            "Somewhere",
            "Prayer keeps its own"
        );
    }

    #[test]
    fn a_fetch_that_finishes_after_the_place_moved_is_not_cached() {
        let store = store_with_place();
        // The fetch for the first place was under way when the user chose another.
        store
            .save_weather_location(&place(1.55, 110.35))
            .expect("saved");
        let written = store
            .write_weather(3.15, 101.69, at("2026-11-26T11:00:00Z"), "{}")
            .expect("not an error");
        assert!(!written, "the old place's sky is not written");
        assert_eq!(store.weather().expect("reads"), None);
        assert!(
            store
                .write_weather(1.55, 110.35, at("2026-11-26T11:00:00Z"), "{}")
                .expect("written"),
            "the new place's is"
        );
    }

    #[test]
    fn a_damaged_time_is_the_same_as_no_cache() {
        let store = store_with_place();
        store
            .write_weather(3.15, 101.69, at("2026-11-26T11:00:00Z"), "{}")
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
                .write_weather(3.15, 101.69, at("2026-11-26T11:00:00Z"), r#"{"kept":true}"#)
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
