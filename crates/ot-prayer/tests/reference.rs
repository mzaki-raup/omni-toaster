//! The reference check (FEAT-0063, Phase 5 exit criterion "live JAKIM times
//! match the official portal on the same dates for at least three zones").
//!
//! `fixtures/jakim_reference.csv` holds three zones × five dates read by hand
//! from the portal's own page. The first test reads the captured responses with
//! the app's parser and compares them to it, so it runs every time; the second
//! asks the real service, and is `#[ignore]` so an unreachable e-Solat is a
//! failure you chose to run, never a pass that proved nothing
//! (`cargo test -p ot-prayer --test reference -- --ignored`).

use jiff::civil::{Date, Time};
use ot_prayer::jakim::{self, Prayer, PrayerDay, Response};

const REFERENCE: &str = include_str!("fixtures/jakim_reference.csv");

/// The prayers the portal shows, in the CSV's column order.
const COLUMNS: [Prayer; 7] = [
    Prayer::Imsak,
    Prayer::Fajr,
    Prayer::Syuruk,
    Prayer::Dhuhr,
    Prayer::Asr,
    Prayer::Maghrib,
    Prayer::Isha,
];

struct Row {
    zone: String,
    date: Date,
    times: Vec<Time>,
}

fn rows() -> Vec<Row> {
    REFERENCE
        .lines()
        .filter(|line| !line.starts_with('#') && !line.starts_with("zone,"))
        .map(|line| {
            let cells: Vec<&str> = line.split(',').collect();
            assert_eq!(cells.len(), 2 + COLUMNS.len(), "a malformed row: {line}");
            Row {
                zone: cells[0].to_owned(),
                date: cells[1].parse().expect("a date"),
                times: cells[2..]
                    .iter()
                    .map(|t| t.parse().expect("a time"))
                    .collect(),
            }
        })
        .collect()
}

/// Every difference between a day and its reference row.
fn differences(row: &Row, day: &PrayerDay) -> Vec<String> {
    COLUMNS
        .iter()
        .zip(&row.times)
        .filter(|(prayer, want)| day.time(**prayer) != **want)
        .map(|(prayer, want)| {
            format!(
                "{} {} {prayer:?}: portal {want}, read {}",
                row.zone,
                row.date,
                day.time(*prayer)
            )
        })
        .collect()
}

#[test]
fn the_reference_table_covers_three_zones_and_five_dates() {
    let rows = rows();
    assert_eq!(rows.len(), 15);
    for zone in ["SGR01", "WLY01", "JHR01"] {
        let dates: Vec<_> = rows.iter().filter(|r| r.zone == zone).collect();
        assert_eq!(dates.len(), 5, "{zone}");
    }
}

#[test]
fn the_captured_responses_read_as_the_portal_shows_them() {
    let mut wrong = Vec::new();
    let mut checked = 0;
    for zone in ["SGR01", "WLY01", "JHR01"] {
        let path = format!(
            "{}/tests/fixtures/esolat-2026-11-20/{zone}-month.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let body = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let Response::Days { days, .. } = jakim::parse(&body).expect("reads") else {
            panic!("{zone}: no record");
        };
        for row in rows().iter().filter(|r| r.zone == zone) {
            let day = days
                .iter()
                .find(|d| d.date == row.date)
                .unwrap_or_else(|| panic!("{zone} {} is not in the capture", row.date));
            wrong.extend(differences(row, day));
            checked += 1;
        }
    }
    assert_eq!(checked, 15);
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
#[ignore = "needs e-Solat"]
fn a_live_fetch_matches_the_portal_for_three_zones() {
    let client = ot_prayer::fetch::Client::new();
    let mut wrong = Vec::new();
    for zone in ["SGR01", "WLY01", "JHR01"] {
        let rows: Vec<Row> = rows().into_iter().filter(|r| r.zone == zone).collect();
        let range = ot_prayer::fetch::Range {
            start: rows.iter().map(|r| r.date).min().expect("rows"),
            end: rows.iter().map(|r| r.date).max().expect("rows"),
        };
        let fetched = client
            .fetch(zone, range, &ot_prayer::fetch::Cancel::new())
            .unwrap_or_else(|error| panic!("{zone}: {error}"));
        for row in &rows {
            let day = fetched
                .days
                .iter()
                .find(|d| d.date == row.date)
                .unwrap_or_else(|| panic!("{zone} {} was not served", row.date));
            wrong.extend(differences(row, day));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
