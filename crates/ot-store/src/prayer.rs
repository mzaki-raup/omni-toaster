//! The prayer-times cache (FEAT-0053, `docs/11-prayer-and-azan.md` §4).
//!
//! What a fetch brought back, kept so the app has times with the network off.
//! The types here are plain values, not `ot_prayer`'s: service crates meet only
//! in the binary, so the binary turns a fetched day into a [`CachedDay`] and
//! back. The eight times are in the order e-Solat lists them.

use jiff::civil::{Date, Time};
use jiff::{SignedDuration, Timestamp};
use rusqlite::types::Type;
use rusqlite::{OptionalExtension as _, Row, params};

use crate::reminders::stamp;
use crate::{Store, StoreError};

/// A cached day is fresh while it was written less than this long ago.
pub const FRESH_FOR: SignedDuration = SignedDuration::from_hours(30 * 24);

/// One day's times for one zone: imsak, fajr, syuruk, dhuha, dhuhr, asr,
/// maghrib, isha — Malaysian clock values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedDay {
    pub date: Date,
    pub hijri: String,
    pub times: [Time; 8],
}

/// Where a row came from: the `source` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A live e-Solat fetch.
    Api,
    /// The dataset shipped in the exe (spec §5).
    Bundled,
}

impl Origin {
    fn as_str(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Bundled => "bundled",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "api" => Some(Self::Api),
            "bundled" => Some(Self::Bundled),
            _ => None,
        }
    }
}

/// A row as read: the day, where it came from and when it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cached {
    pub day: CachedDay,
    pub origin: Origin,
    pub fetched_at: Timestamp,
}

/// Whether the cache can serve a zone today, and if not, why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    /// No row for today, or no rows at all.
    NoEntryForToday,
    /// Today's row was written [`FRESH_FOR`] or longer ago.
    TooOld,
}

/// The time columns, in the order of [`CachedDay::times`].
const TIME_COLUMNS: &str = "imsak, fajr, syuruk, dhuha, dhuhr, asr, maghrib, isha";

/// A stored value that does not read back as what it should be: the row is
/// damaged, and that is an error to report, not a day to skip.
fn damaged<E>(index: usize, error: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
}

fn read_time(row: &Row<'_>, index: usize) -> rusqlite::Result<Time> {
    let text: String = row.get(index)?;
    text.parse().map_err(|error| damaged(index, error))
}

fn read_row(row: &Row<'_>) -> rusqlite::Result<Cached> {
    let date: String = row.get(0)?;
    let mut times = [Time::midnight(); 8];
    for (slot, time) in times.iter_mut().enumerate() {
        *time = read_time(row, 2 + slot)?;
    }
    let fetched_at: String = row.get(10)?;
    let origin: String = row.get(11)?;
    Ok(Cached {
        day: CachedDay {
            date: date.parse().map_err(|error| damaged(0, error))?,
            hijri: row.get(1)?,
            times,
        },
        origin: Origin::parse(&origin).ok_or_else(|| {
            damaged(
                11,
                std::io::Error::other(format!("unknown source {origin:?}")),
            )
        })?,
        fetched_at: fetched_at.parse().map_err(|error| damaged(10, error))?,
    })
}

impl Store {
    /// Write a fetched range for a zone in one transaction. A day already
    /// there is **replaced**, not skipped: e-Solat corrects its tables, and the
    /// newest fetch is the one to believe. The zone code is kept upper case.
    pub fn write_prayer_days(
        &self,
        zone: &str,
        days: &[CachedDay],
        origin: Origin,
        fetched_at: Timestamp,
    ) -> Result<(), StoreError> {
        let zone = zone.to_ascii_uppercase();
        let fetched_at = stamp(fetched_at);
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut insert = tx.prepare(&format!(
                "INSERT INTO jakim_prayer_cache \
                 (zone_code, date, hijri, {TIME_COLUMNS}, fetched_at, source) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) \
                 ON CONFLICT (zone_code, date) DO UPDATE SET \
                 hijri = excluded.hijri, imsak = excluded.imsak, fajr = excluded.fajr, \
                 syuruk = excluded.syuruk, dhuha = excluded.dhuha, dhuhr = excluded.dhuhr, \
                 asr = excluded.asr, maghrib = excluded.maghrib, isha = excluded.isha, \
                 fetched_at = excluded.fetched_at, source = excluded.source"
            ))?;
            for day in days {
                let at = |n: usize| day.times[n].strftime("%H:%M:%S").to_string();
                insert.execute(params![
                    zone,
                    day.date.to_string(),
                    day.hijri,
                    at(0),
                    at(1),
                    at(2),
                    at(3),
                    at(4),
                    at(5),
                    at(6),
                    at(7),
                    fetched_at,
                    origin.as_str(),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// One zone's day, if the cache has it.
    pub fn prayer_day(&self, zone: &str, date: Date) -> Result<Option<Cached>, StoreError> {
        Ok(self
            .conn
            .query_row(
                &format!(
                    "SELECT date, hijri, {TIME_COLUMNS}, fetched_at, source \
                     FROM jakim_prayer_cache WHERE zone_code = ?1 AND date = ?2"
                ),
                params![zone.to_ascii_uppercase(), date.to_string()],
                read_row,
            )
            .optional()?)
    }

    /// Whether the cache can serve `zone` on `today`: it has the day, and the
    /// row was written less than [`FRESH_FOR`] before `now`. A row stamped in
    /// the future — the clock was set back — counts as fresh.
    pub fn prayer_freshness(
        &self,
        zone: &str,
        today: Date,
        now: Timestamp,
    ) -> Result<Freshness, StoreError> {
        let Some(cached) = self.prayer_day(zone, today)? else {
            return Ok(Freshness::NoEntryForToday);
        };
        Ok(if now.duration_since(cached.fetched_at) < FRESH_FOR {
            Freshness::Fresh
        } else {
            Freshness::TooOld
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::{date, time};

    fn day(d: Date, shift: i8) -> CachedDay {
        CachedDay {
            date: d,
            hijri: "1448-06-10".into(),
            times: [
                time(5 + shift, 39, 0, 0),
                time(5 + shift, 49, 0, 0),
                time(6 + shift, 58, 0, 0),
                time(7 + shift, 23, 0, 0),
                time(13, 2, 0, 0),
                time(16, 24, 0, 0),
                time(18, 59, 0, 0),
                time(20, 12, 0, 0),
            ],
        }
    }

    fn at(text: &str) -> Timestamp {
        text.parse().expect("an instant")
    }

    #[test]
    fn a_range_round_trips_with_its_source_and_time() {
        let store = Store::open_in_memory().expect("opens");
        let fetched = at("2026-11-26T01:00:00Z");
        let days: Vec<_> = (26..=30).map(|d| day(date(2026, 11, d), 0)).collect();
        store
            .write_prayer_days("SGR01", &days, Origin::Api, fetched)
            .expect("written");

        for expected in &days {
            let cached = store
                .prayer_day("SGR01", expected.date)
                .expect("read")
                .expect("the day is there");
            assert_eq!(&cached.day, expected);
            assert_eq!(cached.origin, Origin::Api);
            assert_eq!(cached.fetched_at, fetched);
        }
        assert_eq!(
            store.prayer_day("SGR01", date(2026, 12, 1)).expect("read"),
            None,
            "a day outside the range is absent"
        );
        assert_eq!(
            store.prayer_day("WLY01", date(2026, 11, 26)).expect("read"),
            None,
            "another zone's day is absent"
        );
    }

    #[test]
    fn a_zone_code_is_kept_upper_case_whatever_the_caller_sends() {
        let store = Store::open_in_memory().expect("opens");
        let d = date(2026, 11, 26);
        store
            .write_prayer_days(
                "sgr01",
                &[day(d, 0)],
                Origin::Api,
                at("2026-11-26T01:00:00Z"),
            )
            .expect("written");
        assert!(store.prayer_day("SGR01", d).expect("read").is_some());
        assert!(store.prayer_day("sgr01", d).expect("read").is_some());
    }

    #[test]
    fn a_refetch_replaces_a_corrected_day_and_keeps_the_others() {
        let store = Store::open_in_memory().expect("opens");
        let (d1, d2) = (date(2026, 11, 26), date(2026, 11, 27));
        store
            .write_prayer_days(
                "SGR01",
                &[day(d1, 0), day(d2, 0)],
                Origin::Bundled,
                at("2026-11-01T00:00:00Z"),
            )
            .expect("first");

        // e-Solat corrects the 26th, an hour later in this made-up case.
        let corrected = day(d1, 1);
        store
            .write_prayer_days(
                "SGR01",
                std::slice::from_ref(&corrected),
                Origin::Api,
                at("2026-11-26T01:00:00Z"),
            )
            .expect("second");

        let first = store.prayer_day("SGR01", d1).expect("read").expect("there");
        assert_eq!(first.day, corrected, "the corrected times replaced the old");
        assert_eq!(first.origin, Origin::Api);
        assert_eq!(first.fetched_at, at("2026-11-26T01:00:00Z"));

        let second = store.prayer_day("SGR01", d2).expect("read").expect("there");
        assert_eq!(second.day, day(d2, 0), "the day not refetched is untouched");
        assert_eq!(second.origin, Origin::Bundled);

        let rows: i64 = store
            .connection()
            .query_row("SELECT COUNT(*) FROM jakim_prayer_cache", [], |r| r.get(0))
            .expect("count");
        assert_eq!(rows, 2, "a replaced day is not duplicated");
    }

    #[test]
    fn freshness_is_whether_today_is_there_and_it_is_under_thirty_days_old() {
        let store = Store::open_in_memory().expect("opens");
        let today = date(2026, 11, 26);
        let fetched = at("2026-11-01T00:00:00Z");
        assert_eq!(
            store
                .prayer_freshness("SGR01", today, fetched)
                .expect("read"),
            Freshness::NoEntryForToday,
            "an empty cache has no entry for today"
        );

        store
            .write_prayer_days("SGR01", &[day(today, 0)], Origin::Api, fetched)
            .expect("written");
        let age = |hours: i64| fetched + SignedDuration::from_hours(hours);
        let check = |now| store.prayer_freshness("SGR01", today, now).expect("read");

        assert_eq!(check(fetched), Freshness::Fresh);
        assert_eq!(
            check(age(30 * 24 - 1)),
            Freshness::Fresh,
            "29 days 23 hours"
        );
        assert_eq!(check(age(30 * 24)), Freshness::TooOld, "exactly 30 days");
        assert_eq!(check(age(31 * 24)), Freshness::TooOld);
        assert_eq!(
            check(fetched - SignedDuration::from_hours(5)),
            Freshness::Fresh,
            "a clock set back does not make the cache stale"
        );

        // Fresh data for other days is no help on a day it lacks.
        assert_eq!(
            store
                .prayer_freshness("SGR01", date(2026, 11, 27), fetched)
                .expect("read"),
            Freshness::NoEntryForToday
        );
        assert_eq!(
            store
                .prayer_freshness("WLY01", today, fetched)
                .expect("read"),
            Freshness::NoEntryForToday,
            "freshness is per zone"
        );
    }

    #[test]
    fn a_damaged_row_is_an_error_not_a_missing_day() {
        let store = Store::open_in_memory().expect("opens");
        let d = date(2026, 11, 26);
        store
            .write_prayer_days(
                "SGR01",
                &[day(d, 0)],
                Origin::Api,
                at("2026-11-26T01:00:00Z"),
            )
            .expect("written");
        store
            .connection()
            .execute("UPDATE jakim_prayer_cache SET fajr = 'later'", [])
            .expect("damage");
        assert!(store.prayer_day("SGR01", d).is_err());
    }

    #[test]
    fn a_source_other_than_api_or_bundled_is_refused_by_the_table() {
        let store = Store::open_in_memory().expect("opens");
        let result = store.connection().execute(
            "INSERT INTO jakim_prayer_cache VALUES \
             ('SGR01','2026-11-26','h','05:00:00','05:00:00','05:00:00','05:00:00',\
              '05:00:00','05:00:00','05:00:00','05:00:00','2026-11-26T00:00:00.000Z','guess')",
            [],
        );
        assert!(result.is_err());
    }
}
