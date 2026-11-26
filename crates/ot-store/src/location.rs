//! The one saved location (FEAT-0056, `docs/11-prayer-and-azan.md` §6).
//!
//! A single row in `location`: where the user is, how the app knows, and when it
//! last looked. The row is also the cache of the daily IP lookup — a detected
//! place written less than a day ago is not looked up again — so there is no
//! separate cache file. A place the user typed or chose is `manual`, and a
//! detection never overwrites it.

use jiff::Timestamp;
use rusqlite::{OptionalExtension as _, params};

use crate::reminders::stamp;
use crate::{Store, StoreError};

/// The row's id: there is only ever one.
const ROW: i64 = 1;

/// How the app came to know the place; the `source` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Looked up from the PC's IP address, with consent.
    Ip,
    /// Typed or chosen by the user.
    Manual,
}

impl Origin {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ip => "ip",
            Self::Manual => "manual",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "ip" => Some(Self::Ip),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }
}

/// The saved place. Coordinates are already rounded by the caller
/// (`ot_prayer::location::round`); the store keeps what it is given.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredLocation {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    /// An IANA zone name.
    pub tz: String,
    pub country_code: Option<String>,
    pub origin: Origin,
    pub updated_at: Timestamp,
}

fn damaged<E>(index: usize, error: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, Box::new(error))
}

impl Store {
    fn write_location(&self, place: &StoredLocation) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO location (id, name, lat, lon, tz, country_code, source, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
             ON CONFLICT (id) DO UPDATE SET name = excluded.name, lat = excluded.lat, \
             lon = excluded.lon, tz = excluded.tz, country_code = excluded.country_code, \
             source = excluded.source, updated_at = excluded.updated_at",
            params![
                ROW,
                place.name,
                place.latitude,
                place.longitude,
                place.tz,
                place.country_code,
                place.origin.as_str(),
                stamp(place.updated_at),
            ],
        )?;
        Ok(())
    }

    /// Save a place the user typed or chose. It replaces whatever was there,
    /// detected or not, and from then on the app does not re-detect.
    pub fn save_manual_location(&self, place: &StoredLocation) -> Result<(), StoreError> {
        self.write_location(&StoredLocation {
            origin: Origin::Manual,
            ..place.clone()
        })
    }

    /// Save a place the IP lookup found — unless the user has set one by hand,
    /// which a detection never overrides. Returns whether it was written.
    pub fn save_detected_location(&self, place: &StoredLocation) -> Result<bool, StoreError> {
        if self
            .location()?
            .is_some_and(|saved| saved.origin == Origin::Manual)
        {
            return Ok(false);
        }
        self.write_location(&StoredLocation {
            origin: Origin::Ip,
            ..place.clone()
        })?;
        Ok(true)
    }

    /// The saved place, if there is one.
    pub fn location(&self) -> Result<Option<StoredLocation>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT name, lat, lon, tz, country_code, source, updated_at \
                 FROM location WHERE id = ?1",
                params![ROW],
                |row| {
                    let origin: String = row.get(5)?;
                    let updated_at: String = row.get(6)?;
                    Ok(StoredLocation {
                        name: row.get(0)?,
                        latitude: row.get(1)?,
                        longitude: row.get(2)?,
                        tz: row.get(3)?,
                        country_code: row.get(4)?,
                        origin: Origin::parse(&origin).ok_or_else(|| {
                            damaged(
                                5,
                                std::io::Error::other(format!("unknown source {origin:?}")),
                            )
                        })?,
                        updated_at: updated_at.parse().map_err(|error| damaged(6, error))?,
                    })
                },
            )
            .optional()?)
    }

    /// Forget the saved place, so the next run asks again.
    pub fn clear_location(&self) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM location WHERE id = ?1", params![ROW])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(name: &str, origin: Origin, at: &str) -> StoredLocation {
        StoredLocation {
            name: name.to_string(),
            latitude: 3.09,
            longitude: 101.53,
            tz: "Asia/Kuala_Lumpur".to_string(),
            country_code: Some("MY".to_string()),
            origin,
            updated_at: at.parse().expect("an instant"),
        }
    }

    #[test]
    fn there_is_no_location_until_one_is_saved_and_it_reads_back_whole() {
        let store = Store::open_in_memory().expect("opens");
        assert_eq!(store.location().expect("read"), None);

        let shah_alam = place("Shah Alam", Origin::Ip, "2026-11-26T01:00:00Z");
        assert!(store.save_detected_location(&shah_alam).expect("saved"));
        assert_eq!(store.location().expect("read"), Some(shah_alam));
    }

    #[test]
    fn a_manual_place_replaces_a_detected_one_and_a_detection_never_replaces_it_back() {
        let store = Store::open_in_memory().expect("opens");
        store
            .save_detected_location(&place("Detected", Origin::Ip, "2026-11-25T01:00:00Z"))
            .expect("saved");

        // Typed by hand, whatever origin the caller put on the value.
        store
            .save_manual_location(&place("Mine", Origin::Ip, "2026-11-26T01:00:00Z"))
            .expect("saved");
        let saved = store.location().expect("read").expect("a place");
        assert_eq!(
            (saved.name.as_str(), saved.origin),
            ("Mine", Origin::Manual)
        );

        // A later detection is refused and changes nothing.
        let written = store
            .save_detected_location(&place("Detected again", Origin::Ip, "2026-11-27T01:00:00Z"))
            .expect("not an error");
        assert!(!written);
        assert_eq!(store.location().expect("read"), Some(saved));
    }

    #[test]
    fn a_detection_replaces_an_older_detection_and_one_row_is_all_there_is() {
        let store = Store::open_in_memory().expect("opens");
        store
            .save_detected_location(&place("Old", Origin::Ip, "2026-11-25T01:00:00Z"))
            .expect("saved");
        store
            .save_detected_location(&place("New", Origin::Ip, "2026-11-26T01:00:00Z"))
            .expect("saved");
        let rows: i64 = store
            .connection()
            .query_row("SELECT COUNT(*) FROM location", [], |r| r.get(0))
            .expect("count");
        assert_eq!(rows, 1);
        assert_eq!(
            store.location().expect("read").expect("a place").name,
            "New"
        );
    }

    #[test]
    fn clearing_forgets_a_manual_place_too_so_detection_may_run_again() {
        let store = Store::open_in_memory().expect("opens");
        store
            .save_manual_location(&place("Mine", Origin::Manual, "2026-11-26T01:00:00Z"))
            .expect("saved");
        store.clear_location().expect("cleared");
        assert_eq!(store.location().expect("read"), None);
        assert!(
            store
                .save_detected_location(&place("Detected", Origin::Ip, "2026-11-27T01:00:00Z"))
                .expect("saved")
        );
    }

    #[test]
    fn a_damaged_row_is_an_error_not_an_absent_place() {
        let store = Store::open_in_memory().expect("opens");
        store
            .save_manual_location(&place("Mine", Origin::Manual, "2026-11-26T01:00:00Z"))
            .expect("saved");
        store
            .connection()
            .execute("UPDATE location SET updated_at = 'yesterday'", [])
            .expect("damage");
        assert!(store.location().is_err());
    }
}
