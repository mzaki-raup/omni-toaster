//! Calculated prayer times (FEAT-0054, ADR-020, `docs/11-prayer-and-azan.md` §8).
//!
//! The offline and overseas source, and the last resort when e-Solat and the
//! cache both fail. It turns a date and a place into the six times of the day
//! with the `adhan` crate. Times come back as instants, so no zone is guessed
//! here: a caller that wants a clock reads the instant in the place's own zone.

use adhan::{Madhab, Method, Parameters, Rounding};
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;

/// A calculation method the user can choose (spec §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// The default outside Malaysia.
    MuslimWorldLeague,
    /// Islamic Society of North America.
    Isna,
    UmmAlQura,
    /// "Malaysia (JAKIM angles, approximate)": Fajr 18.0° and Isha 18.0°. JAKIM's
    /// published times sit within three minutes of these angles (every time, in
    /// SGR01, WLY01 and JHR01, in November); a Fajr of 20.0°, as the spec first
    /// said, is ten minutes early. It follows JAKIM's angles, not its tables, so
    /// it is always labelled approximate.
    MalaysiaJakim,
}

/// The Asar school. Shafi'i is JAKIM's convention and the default; Hanafi puts
/// Asar about an hour later, so a mismatch silently changes a prayer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Asr {
    #[default]
    Shafii,
    Hanafi,
}

/// A place, in degrees: north and east are positive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    pub latitude: f64,
    pub longitude: f64,
}

/// One day's times. A time is `None` where the sun does not reach the angle it
/// needs — inside the polar circles — and is never a made-up number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalculatedDay {
    pub date: Date,
    pub fajr: Option<Timestamp>,
    pub syuruk: Option<Timestamp>,
    pub dhuhr: Option<Timestamp>,
    pub asr: Option<Timestamp>,
    pub maghrib: Option<Timestamp>,
    pub isha: Option<Timestamp>,
}

impl Preset {
    fn parameters(self, asr: Asr) -> Parameters {
        let mut parameters = match self {
            Self::MuslimWorldLeague => Parameters::for_method(Method::MuslimWorldLeague),
            Self::Isna => Parameters::for_method(Method::NorthAmerica),
            Self::UmmAlQura => Parameters::for_method(Method::UmmAlQura),
            Self::MalaysiaJakim => {
                let mut parameters = Parameters::for_method(Method::Other);
                parameters.fajr_angle = 18.0;
                parameters.isha_angle = 18.0;
                parameters
            }
        };
        parameters.madhab = match asr {
            Asr::Shafii => Madhab::Shafi,
            Asr::Hanafi => Madhab::Hanafi,
        };
        parameters.rounding = Rounding::Nearest;
        parameters
    }
}

/// Whether times from this preset are labelled "approximate" (spec §8): the
/// Malaysia preset always is, and any preset is when it stands in for JAKIM in
/// Malaysia.
pub fn is_approximate(preset: Preset, in_malaysia: bool) -> bool {
    in_malaysia || preset == Preset::MalaysiaJakim
}

/// `adhan` gives a time as a UTC clock reading with the day taken off, so the
/// reading is placed on whichever of the neighbouring UTC days lands it nearest
/// the place's solar noon. Every prayer is within twelve hours of that.
fn place_on_day(
    reading: Option<(i64, u32, u32)>,
    midnight_utc: i64,
    noon: i64,
) -> Option<Timestamp> {
    let (hour, minute, second) = reading?;
    let clock = midnight_utc + hour * 3600 + i64::from(minute) * 60 + i64::from(second);
    let instant = (-1..=1)
        .map(|day| clock + day * 86_400)
        .min_by_key(|candidate| (candidate - noon).abs())?;
    Timestamp::from_second(instant).ok()
}

/// The times for `date` at `place`. `date` is the calendar day at the place.
pub fn day(date: Date, place: Place, preset: Preset, asr: Asr) -> CalculatedDay {
    let times = adhan::prayer_times(
        i32::from(date.year()),
        u32::try_from(date.month()).unwrap_or(1),
        u32::try_from(date.day()).unwrap_or(1),
        place.latitude,
        place.longitude,
        &preset.parameters(asr),
    );
    // Midnight UTC of the date, and the instant of solar noon there: 12:00 UTC
    // moved west by one hour per 15 degrees east.
    let midnight_utc = date
        .at(0, 0, 0, 0)
        .to_zoned_datetime(TimeZone::UTC)
        .map_or(0, |zoned| zoned.timestamp().as_second());
    let noon = midnight_utc + 12 * 3600 - (place.longitude / 15.0 * 3600.0).round() as i64;
    let at = |reading| place_on_day(reading, midnight_utc, noon);
    CalculatedDay {
        date,
        fajr: at(times.fajr),
        syuruk: at(times.sunrise),
        dhuhr: at(times.dhuhr),
        asr: at(times.asr),
        maghrib: at(times.maghrib),
        isha: at(times.isha),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jakim::{self, Prayer, PrayerDay, Response};

    const SGR01: &str = include_str!("../tests/fixtures/esolat-2026-11-20/SGR01-month.json");
    const WLY01: &str = include_str!("../tests/fixtures/esolat-2026-11-20/WLY01-month.json");
    const JHR01: &str = include_str!("../tests/fixtures/esolat-2026-11-20/JHR01-month.json");
    const PRESET_ROWS: &str = include_str!("../tests/fixtures/adhan_js_presets.tsv");

    /// A point in each zone, from `data/jakim_zone_points.csv`.
    const ZONES: [(&str, &str, Place); 3] = [
        (
            "SGR01",
            SGR01,
            Place {
                latitude: 3.0851,
                longitude: 101.5328,
            },
        ),
        (
            "WLY01",
            WLY01,
            Place {
                latitude: 3.1412,
                longitude: 101.6865,
            },
        ),
        (
            "JHR01",
            JHR01,
            Place {
                latitude: 2.4497,
                longitude: 104.5211,
            },
        ),
    ];

    fn days(body: &str) -> Vec<PrayerDay> {
        match jakim::parse(body.as_bytes()).expect("parses") {
            Response::Days { days, .. } => days,
            Response::NoRecord => panic!("no record"),
        }
    }

    fn all(day: &CalculatedDay) -> [Option<Timestamp>; 6] {
        [
            day.fajr,
            day.syuruk,
            day.dhuhr,
            day.asr,
            day.maghrib,
            day.isha,
        ]
    }

    fn seconds_of_utc_day(instant: Timestamp) -> i64 {
        instant.as_second().rem_euclid(86_400)
    }

    #[test]
    fn the_malaysia_preset_is_within_three_minutes_of_jakim_in_three_zones() {
        // November 2026, 30 days in each of SGR01, WLY01 and JHR01, as the
        // spike captured them. Measured: Fajr +0 to +3 (JAKIM later), Syuruk
        // -3 to +1, Dhuhr +2 to +3, Asar +1 to +2, Maghrib +0 to +2, Isha +0
        // to +2 — JAKIM adds a precaution the angles do not. At a Fajr of
        // 20.0 degrees the first column is +10 to +12, which is why it is 18.
        let prayers = [
            Prayer::Fajr,
            Prayer::Syuruk,
            Prayer::Dhuhr,
            Prayer::Asr,
            Prayer::Maghrib,
            Prayer::Isha,
        ];
        for (code, body, place) in ZONES {
            let published = days(body);
            assert_eq!(published.len(), 30, "{code}");
            for jakim_day in published {
                let calculated = day(jakim_day.date, place, Preset::MalaysiaJakim, Asr::Shafii);
                for (prayer, mine) in prayers.iter().zip(all(&calculated)) {
                    let theirs = jakim_day.at(*prayer).expect("JAKIM's time");
                    let off =
                        (theirs.as_second() - mine.expect("a time").as_second()) as f64 / 60.0;
                    assert!(
                        off.abs() <= 3.0,
                        "{code} {} {prayer:?}: JAKIM is {off} minutes from the calculation",
                        jakim_day.date
                    );
                }
            }
        }
    }

    #[test]
    fn shafii_and_hanafi_differ_on_asar_alone_and_by_about_an_hour() {
        let (_, body, place) = ZONES[0];
        for jakim_day in days(body) {
            let shafii = day(jakim_day.date, place, Preset::MalaysiaJakim, Asr::Shafii);
            let hanafi = day(jakim_day.date, place, Preset::MalaysiaJakim, Asr::Hanafi);
            let gap = hanafi.asr.expect("asr").as_second() - shafii.asr.expect("asr").as_second();
            assert!(
                (50 * 60..=60 * 60).contains(&gap),
                "{}: Hanafi Asar is {} minutes after Shafi'i",
                jakim_day.date,
                gap / 60
            );
            assert_eq!(
                (
                    shafii.fajr,
                    shafii.syuruk,
                    shafii.dhuhr,
                    shafii.maghrib,
                    shafii.isha
                ),
                (
                    hanafi.fajr,
                    hanafi.syuruk,
                    hanafi.dhuhr,
                    hanafi.maghrib,
                    hanafi.isha
                ),
                "{}: only Asar moves",
                jakim_day.date
            );
        }
        assert_eq!(Asr::default(), Asr::Shafii, "Shafi'i is the default");
    }

    #[test]
    fn the_other_presets_match_the_reference_rows() {
        let mut compared = 0;
        for line in PRESET_ROWS.lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<&str> = line.split('\t').collect();
            assert_eq!(f.len(), 13, "{line}");
            let place = Place {
                latitude: f[0].parse().expect("latitude"),
                longitude: f[1].parse().expect("longitude"),
            };
            let date = Date::new(
                f[2].parse().expect("year"),
                f[3].parse().expect("month"),
                f[4].parse().expect("day"),
            )
            .expect("a date");
            let preset = match f[5] {
                "MuslimWorldLeague" => Preset::MuslimWorldLeague,
                "NorthAmerica" => Preset::Isna,
                "UmmAlQura" => Preset::UmmAlQura,
                other => panic!("unexpected method {other}"),
            };
            let asr = if f[6] == "Hanafi" {
                Asr::Hanafi
            } else {
                Asr::Shafii
            };

            let calculated = day(date, place, preset, asr);
            let times = all(&calculated);
            for (name, (reading, mine)) in ["fajr", "syuruk", "dhuhr", "asr", "maghrib", "isha"]
                .iter()
                .zip(f[7..].iter().zip(times))
            {
                let mut parts = reading
                    .split(':')
                    .map(|n| n.parse::<i64>().expect("a number"));
                let expected = parts.next().expect("h") * 3600
                    + parts.next().expect("m") * 60
                    + parts.next().expect("s");
                let got = seconds_of_utc_day(mine.expect("a time"));
                assert!(
                    (got - expected).abs() <= 60,
                    "{line}: {name} is {got}s into the UTC day, the reference says {expected}s"
                );
                compared += 1;
            }
            // Placed on the right day: the six are in the order of the day,
            // which fails if a reading lands on the wrong side of midnight UTC
            // (Sydney's Fajr, New York's Isha).
            let instants: Vec<_> = times.iter().map(|t| t.expect("a time")).collect();
            assert!(
                instants.windows(2).all(|pair| pair[0] < pair[1]),
                "{line}: the times are not in order: {instants:?}"
            );
        }
        assert_eq!(compared, 60 * 6, "every row, every time");
    }

    #[test]
    fn a_malaysian_day_falls_on_its_own_date_in_malaysia() {
        // Fajr at 05:49 in Malaysia is 21:49 UTC the evening before, the case
        // a UTC clock reading with no date gets wrong.
        let kl = jiff::tz::TimeZone::fixed(jiff::tz::offset(8));
        let (_, body, place) = ZONES[1];
        for jakim_day in days(body) {
            let calculated = day(jakim_day.date, place, Preset::MalaysiaJakim, Asr::Shafii);
            for instant in all(&calculated) {
                let local = instant
                    .expect("a time")
                    .to_zoned_datetime(kl.clone())
                    .date();
                assert_eq!(local, jakim_day.date);
            }
        }
    }

    #[test]
    fn where_the_sun_does_not_set_there_is_no_time_not_an_invented_one() {
        // Tromso at midsummer: no sunrise, and Dhuhr still exists.
        let tromso = Place {
            latitude: 69.6492,
            longitude: 18.9553,
        };
        let june = day(
            Date::new(2026, 6, 21).expect("a date"),
            tromso,
            Preset::MuslimWorldLeague,
            Asr::Shafii,
        );
        assert_eq!(june.syuruk, None);
        assert!(june.dhuhr.is_some());
    }

    #[test]
    fn approximate_means_the_malaysia_preset_or_standing_in_for_jakim() {
        assert!(is_approximate(Preset::MalaysiaJakim, false));
        assert!(is_approximate(Preset::MalaysiaJakim, true));
        assert!(is_approximate(Preset::MuslimWorldLeague, true));
        assert!(!is_approximate(Preset::MuslimWorldLeague, false));
        assert!(!is_approximate(Preset::UmmAlQura, false));
    }
}
