//! Reading a JAKIM e-Solat response (`docs/11-prayer-and-azan.md` §2,
//! ADR-018).
//!
//! The endpoint is not a documented API, so the parser trusts nothing about
//! it: the body is capped before it is read, it must be JSON whatever its
//! content type says, `status` must be `OK!` with a `prayerTime` array or the
//! answer is "no data" or an error, and a day with any missing or malformed
//! field — or times out of order — is set aside whole, never shown in part.
//! Unknown fields are ignored, so an added field cannot break it.
//!
//! The times are wall-clock times in `Asia/Kuala_Lumpur`, whatever the PC's
//! own zone; [`PrayerDay::at`] makes an instant of one.

use jiff::Timestamp;
use jiff::civil::{Date, Time};
use jiff::tz::TimeZone;
use serde_json::Value;

/// The most a response may be before it is read: a whole year is 77 KB.
pub const MAX_BODY: usize = 256 * 1024;

/// JAKIM's times are this zone's, wherever the PC is.
pub const JAKIM_ZONE: &str = "Asia/Kuala_Lumpur";

/// The times e-Solat gives, in the order of the day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Prayer {
    Imsak,
    Fajr,
    Syuruk,
    Dhuha,
    Dhuhr,
    Asr,
    Maghrib,
    Isha,
}

impl Prayer {
    /// Every one, in the order of the day.
    pub const ALL: [Prayer; 8] = [
        Prayer::Imsak,
        Prayer::Fajr,
        Prayer::Syuruk,
        Prayer::Dhuha,
        Prayer::Dhuhr,
        Prayer::Asr,
        Prayer::Maghrib,
        Prayer::Isha,
    ];

    /// Its field in e-Solat's JSON.
    pub fn field(self) -> &'static str {
        match self {
            Prayer::Imsak => "imsak",
            Prayer::Fajr => "fajr",
            Prayer::Syuruk => "syuruk",
            Prayer::Dhuha => "dhuha",
            Prayer::Dhuhr => "dhuhr",
            Prayer::Asr => "asr",
            Prayer::Maghrib => "maghrib",
            Prayer::Isha => "isha",
        }
    }
}

/// One day's times for a zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrayerDay {
    pub date: Date,
    /// As e-Solat gives it, `1448-06-10`; shown, never computed with.
    pub hijri: String,
    /// In [`Prayer::ALL`] order.
    pub times: [Time; 8],
}

impl PrayerDay {
    /// The wall-clock time of a prayer.
    pub fn time(&self, prayer: Prayer) -> Time {
        self.times[prayer as usize]
    }

    /// The instant of a prayer: its wall-clock time in `Asia/Kuala_Lumpur`,
    /// whatever the PC's own zone (`docs/11-prayer-and-azan.md` §4).
    pub fn at(&self, prayer: Prayer) -> Option<Timestamp> {
        let tz = TimeZone::get(JAKIM_ZONE).ok()?;
        self.date
            .to_datetime(self.time(prayer))
            .to_zoned_datetime(tz)
            .ok()
            .map(|zoned| zoned.timestamp())
    }
}

/// A day set aside, and why. Logged, never shown as times.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedDay {
    /// The day's `date` as sent, or its position when it had none.
    pub date: String,
    pub reason: String,
}

/// What a response said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// `OK!`: the days that read, in the order sent, and those set aside.
    Days {
        zone: String,
        days: Vec<PrayerDay>,
        rejected: Vec<RejectedDay>,
    },
    /// `NO_RECORD!`: nothing for that zone and range — an unknown zone, or a
    /// year JAKIM has not published yet. Not a failure to retry.
    NoRecord,
}

/// Why a response could not be read at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("the response is {0} bytes, over the {MAX_BODY}-byte limit")]
    TooLarge(usize),
    /// An HTML error page, whatever its content type said, or a cut-off body.
    #[error("the response is not JSON: {0}")]
    NotJson(String),
    #[error("e-Solat answered with status {0:?}")]
    Status(String),
    #[error("the response is not the shape e-Solat sends: {0}")]
    Shape(String),
}

/// Read a response body.
pub fn parse(body: &[u8]) -> Result<Response, ParseError> {
    if body.len() > MAX_BODY {
        return Err(ParseError::TooLarge(body.len()));
    }
    let value: Value =
        serde_json::from_slice(body).map_err(|e| ParseError::NotJson(e.to_string()))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Shape("no status".to_owned()))?;
    match status {
        "OK!" => {}
        "NO_RECORD!" => return Ok(Response::NoRecord),
        other => return Err(ParseError::Status(other.to_owned())),
    }
    let zone = value
        .get("zone")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_uppercase();
    let list = value
        .get("prayerTime")
        .and_then(Value::as_array)
        .ok_or_else(|| ParseError::Shape("prayerTime is not a list".to_owned()))?;

    let mut days = Vec::with_capacity(list.len());
    let mut rejected = Vec::new();
    for (index, item) in list.iter().enumerate() {
        match day(item) {
            Ok(day) => days.push(day),
            Err(reason) => rejected.push(RejectedDay {
                date: item
                    .get("date")
                    .and_then(Value::as_str)
                    .map_or_else(|| format!("#{index}"), str::to_owned),
                reason,
            }),
        }
    }
    Ok(Response::Days {
        zone,
        days,
        rejected,
    })
}

/// One element of `prayerTime`, or why it was set aside.
fn day(item: &Value) -> Result<PrayerDay, String> {
    let text = |field: &str| {
        item.get(field)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{field} is missing or not text"))
    };
    let date = parse_date(text("date")?)?;
    let hijri = text("hijri")?.to_owned();
    let mut times = [Time::midnight(); 8];
    for prayer in Prayer::ALL {
        times[prayer as usize] = parse_time(text(prayer.field())?)
            .ok_or_else(|| format!("{} is not a time", prayer.field()))?;
    }
    if times.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("the times are out of order".to_owned());
    }
    Ok(PrayerDay { date, hijri, times })
}

/// `20-Nov-2026`, with the month in Malay (`Mac`, `Mei`, `Ogos`, `Okt`, `Dis`)
/// as e-Solat writes it, or in English.
fn parse_date(text: &str) -> Result<Date, String> {
    let bad = || format!("{text:?} is not a date");
    let mut parts = text.split('-');
    let (Some(day), Some(month), Some(year), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(bad());
    };
    let month = match month.to_ascii_lowercase().as_str() {
        "jan" => 1,
        "feb" => 2,
        "mac" | "mar" => 3,
        "apr" => 4,
        "mei" | "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "ogos" | "ogo" | "aug" => 8,
        "sep" => 9,
        "okt" | "oct" => 10,
        "nov" => 11,
        "dis" | "dec" => 12,
        _ => return Err(bad()),
    };
    let day: i8 = day.parse().map_err(|_| bad())?;
    let year: i16 = year.parse().map_err(|_| bad())?;
    Date::new(year, month, day).map_err(|_| bad())
}

/// `05:49:00`, as e-Solat writes it, or `05:49`.
fn parse_time(text: &str) -> Option<Time> {
    let mut parts = text.split(':');
    let hour: i8 = parts.next()?.parse().ok()?;
    let minute: i8 = parts.next()?.parse().ok()?;
    let second: i8 = match parts.next() {
        Some(second) => second.parse().ok()?,
        None => 0,
    };
    if parts.next().is_some() {
        return None;
    }
    Time::new(hour, minute, second, 0).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::{date, time};

    const SGR01: &str = include_str!("../tests/fixtures/esolat-2026-11-20/SGR01-month.json");
    const WLY01: &str = include_str!("../tests/fixtures/esolat-2026-11-20/WLY01-month.json");
    const JHR01: &str = include_str!("../tests/fixtures/esolat-2026-11-20/JHR01-month.json");
    const DECEMBER: &str =
        include_str!("../tests/fixtures/esolat-2026-11-20/SGR01-duration-december.json");
    const SAMPLE: &str = include_str!("../tests/fixtures/jakim_sample.json");

    fn days(body: &str) -> (String, Vec<PrayerDay>, Vec<RejectedDay>) {
        match parse(body.as_bytes()).expect("parses") {
            Response::Days {
                zone,
                days,
                rejected,
            } => (zone, days, rejected),
            Response::NoRecord => panic!("no record"),
        }
    }

    fn on(days: &[PrayerDay], day: Date) -> &PrayerDay {
        days.iter().find(|d| d.date == day).expect("the day")
    }

    #[test]
    fn the_three_zones_read_whole_with_the_times_the_spike_saw() {
        // The exit criterion "the parser passes the fixtures for three zones":
        // every day of each capture reads, and 20 November says what e-Solat
        // said that morning (ADR-018).
        let expected = [
            (
                SGR01,
                "SGR01",
                [
                    (5, 39),
                    (5, 49),
                    (6, 58),
                    (7, 23),
                    (13, 2),
                    (16, 24),
                    (18, 59),
                    (20, 12),
                ],
            ),
            (
                WLY01,
                "WLY01",
                [
                    (5, 39),
                    (5, 49),
                    (6, 59),
                    (7, 24),
                    (13, 2),
                    (16, 24),
                    (18, 59),
                    (20, 12),
                ],
            ),
            (
                JHR01,
                "JHR01",
                [
                    (5, 26),
                    (5, 36),
                    (6, 48),
                    (7, 15),
                    (12, 50),
                    (16, 12),
                    (18, 49),
                    (20, 2),
                ],
            ),
        ];
        for (body, code, times) in expected {
            let (zone, days, rejected) = days(body);
            assert_eq!(zone, code);
            assert_eq!(days.len(), 30, "{code}: all of November");
            assert!(rejected.is_empty(), "{code}: {rejected:?}");
            let day = on(&days, date(2026, 11, 20));
            assert_eq!(day.hijri, "1448-06-10");
            for (prayer, (h, m)) in Prayer::ALL.into_iter().zip(times) {
                assert_eq!(day.time(prayer), time(h, m, 0, 0), "{code} {prayer:?}");
            }
        }
    }

    #[test]
    fn malay_month_names_read_december_whole() {
        let (_, days, rejected) = days(DECEMBER);
        assert_eq!(days.len(), 31);
        assert!(rejected.is_empty());
        assert_eq!(days[30].date, date(2026, 12, 31), "31-Dis-2026");
    }

    #[test]
    fn the_sample_sets_aside_exactly_the_broken_days_and_says_why() {
        let (zone, days, rejected) = days(SAMPLE);
        assert_eq!(zone, "SGR01");
        let read: Vec<Date> = days.iter().map(|d| d.date).collect();
        assert_eq!(
            read,
            vec![
                date(2026, 11, 20),
                date(2026, 11, 21),
                date(2026, 8, 17),
                date(2026, 8, 18)
            ],
            "the good day, the HH:MM day with an extra field, Ogos and Aug"
        );
        let why: Vec<(&str, &str)> = rejected
            .iter()
            .map(|r| (r.date.as_str(), r.reason.as_str()))
            .collect();
        assert_eq!(
            why,
            vec![
                ("22-Nov-2026", "fajr is missing or not text"),
                ("23-Nov-2026", "fajr is not a time"),
                ("24-Nov-2026", "isha is not a time"),
                ("25-Nov-2026", "isha is missing or not text"),
                ("26-Nov-2026", "the times are out of order"),
                ("31-Nov-2026", "\"31-Nov-2026\" is not a date"),
                ("28-Nob-2026", "\"28-Nob-2026\" is not a date"),
                ("#9", "date is missing or not text"),
            ]
        );
        assert_eq!(
            on(&days, date(2026, 11, 21)).time(Prayer::Maghrib),
            time(19, 0, 0, 0)
        );
    }

    #[test]
    fn no_record_is_no_data_not_an_error() {
        for body in [
            include_str!("../tests/fixtures/esolat-2026-11-20/unknown-zone-no-record.json"),
            include_str!(
                "../tests/fixtures/esolat-2026-11-20/SGR01-duration-next-year-no-record.json"
            ),
        ] {
            assert_eq!(parse(body.as_bytes()), Ok(Response::NoRecord));
        }
    }

    #[test]
    fn an_html_page_labelled_json_is_refused_as_not_json() {
        let html = include_str!(
            "../tests/fixtures/esolat-2026-11-20/duration-as-get-http500-html-body.txt"
        );
        assert!(matches!(
            parse(html.as_bytes()),
            Err(ParseError::NotJson(_))
        ));
        assert!(matches!(parse(b""), Err(ParseError::NotJson(_))));
        assert!(
            matches!(
                parse(&SGR01.as_bytes()[..SGR01.len() / 2]),
                Err(ParseError::NotJson(_))
            ),
            "a body cut off half way"
        );
    }

    #[test]
    fn an_unknown_status_or_shape_is_an_error_naming_it() {
        assert_eq!(
            parse(br#"{"status":"MAINTENANCE","prayerTime":[]}"#),
            Err(ParseError::Status("MAINTENANCE".to_owned()))
        );
        assert!(matches!(
            parse(br#"{"prayerTime":[]}"#),
            Err(ParseError::Shape(_))
        ));
        assert!(matches!(
            parse(br#"{"status":"OK!","prayerTime":{"data":[]}}"#),
            Err(ParseError::Shape(_))
        ));
    }

    #[test]
    fn a_body_over_the_cap_is_refused_before_it_is_read() {
        let big = vec![b' '; MAX_BODY + 1];
        assert_eq!(parse(&big), Err(ParseError::TooLarge(MAX_BODY + 1)));
        // A whole year, the largest real answer, is well inside it.
        const { assert!(77_256 < MAX_BODY) };
    }

    #[test]
    fn a_prayer_is_an_instant_in_kuala_lumpur_whatever_the_pcs_zone() {
        // 05:49 in Kuala Lumpur is 21:49 UTC the day before. The conversion
        // names its zone, so the machine's own setting cannot change it.
        let (_, days, _) = days(SGR01);
        let day = on(&days, date(2026, 11, 20));
        assert_eq!(
            day.at(Prayer::Fajr),
            Some("2026-11-19T21:49:00Z".parse().expect("instant"))
        );
        assert_eq!(
            day.at(Prayer::Isha),
            Some("2026-11-20T12:12:00Z".parse().expect("instant"))
        );
    }

    #[test]
    fn dates_and_times_read_only_in_their_published_forms() {
        assert_eq!(parse_date("20-Nov-2026"), Ok(date(2026, 11, 20)));
        assert_eq!(parse_date("01-mac-2026"), Ok(date(2026, 3, 1)), "any case");
        for bad in [
            "2026-11-20",
            "20/Nov/2026",
            "20-Nov",
            "29-Feb-2026",
            "x-Nov-2026",
        ] {
            assert!(parse_date(bad).is_err(), "{bad}");
        }
        assert_eq!(parse_time("20:12:00"), Some(time(20, 12, 0, 0)));
        assert_eq!(parse_time("20:12"), Some(time(20, 12, 0, 0)));
        for bad in ["24:00", "20:60", "8pm", "20:12:00:00", "", "20"] {
            assert_eq!(parse_time(bad), None, "{bad}");
        }
    }
}
