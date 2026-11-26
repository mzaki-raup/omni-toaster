//! Reading an Open-Meteo answer (FEAT-0065, ADR-023).
//!
//! [`parse`] turns the body into an [`Observation`] or says why it cannot. The
//! service is not ours: every field is checked, a missing or odd one fails the
//! whole answer, extra fields are ignored, and a malformed body is an error
//! and never a panic (`.claude/rules/security.md`, "Network").

use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use serde_json::Value;

/// The most a body may be: a day's hourly series is about 1.2 KB, so this is
/// generous and still stops an endless answer.
pub const MAX_BODY: usize = 64 * 1024;

/// What the temperature check will believe, in °C: past either end is a
/// broken answer, not weather.
const SANE_TEMPERATURE: std::ops::RangeInclusive<f64> = -100.0..=70.0;

/// The current weather and the recent hours, all in UTC.
#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    /// When the service says its current values are for.
    pub at: Timestamp,
    pub temperature_c: f64,
    /// A WMO weather code. Which class it is is the condition classes (FEAT-0066)'s to say.
    pub code: u8,
    /// Hourly temperatures, oldest first. An hour the service had no value for
    /// is left out.
    pub hourly: Vec<(Timestamp, f64)>,
}

/// Why an answer could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("the response is {0} bytes, over the {MAX_BODY}-byte limit")]
    TooLarge(usize),
    /// An empty body, an HTML page or a cut-off answer. Open-Meteo answers
    /// HTTP 200 with an empty body when a request has no coordinates.
    #[error("the response is not JSON: {0}")]
    NotJson(String),
    /// `{"error":true,"reason":"…"}`. The reason is the service's own text: it
    /// is logged and never shown.
    #[error("Open-Meteo refused the request: {0}")]
    Rejected(String),
    #[error("the response is not the shape Open-Meteo sends: {0}")]
    Shape(String),
}

fn shape<T>(what: impl Into<String>) -> Result<T, ParseError> {
    Err(ParseError::Shape(what.into()))
}

/// `2026-11-26T11:15`, as UTC. The request asks for `timezone=GMT`, so an answer
/// in another zone is refused rather than read wrongly.
fn instant(text: &str) -> Result<Timestamp, ParseError> {
    let civil: DateTime = text
        .parse()
        .map_err(|_| ParseError::Shape(format!("{text:?} is not a time")))?;
    civil
        .to_zoned_datetime(TimeZone::UTC)
        .map(|zoned| zoned.timestamp())
        .map_err(|_| ParseError::Shape(format!("{text:?} is not a time")))
}

fn temperature(value: &Value, what: &str) -> Result<f64, ParseError> {
    let Some(degrees) = value.as_f64() else {
        return shape(format!("{what} is not a number"));
    };
    if !SANE_TEMPERATURE.contains(&degrees) {
        return shape(format!("{what} is {degrees}, which is not weather"));
    }
    Ok(degrees)
}

/// Read a response body.
pub fn parse(body: &[u8]) -> Result<Observation, ParseError> {
    if body.len() > MAX_BODY {
        return Err(ParseError::TooLarge(body.len()));
    }
    let root: Value =
        serde_json::from_slice(body).map_err(|error| ParseError::NotJson(error.to_string()))?;
    if root.get("error").and_then(Value::as_bool) == Some(true) {
        let reason = root
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("no reason given");
        return Err(ParseError::Rejected(reason.to_owned()));
    }
    // Asked for GMT, so anything else would put every time in the wrong place.
    match root.get("utc_offset_seconds").and_then(Value::as_i64) {
        Some(0) => {}
        Some(other) => return shape(format!("utc_offset_seconds is {other}, not 0")),
        None => return shape("there is no utc_offset_seconds"),
    }
    let Some(current) = root.get("current") else {
        return shape("there is no current block");
    };
    let at = instant(
        current
            .get("time")
            .and_then(Value::as_str)
            .ok_or_else(|| ParseError::Shape("current has no time".into()))?,
    )?;
    let temperature_c = temperature(
        current.get("temperature_2m").unwrap_or(&Value::Null),
        "current temperature_2m",
    )?;
    let code = current
        .get("weather_code")
        .and_then(Value::as_u64)
        .filter(|code| *code <= 99)
        .ok_or_else(|| ParseError::Shape("current weather_code is not a code".into()))?;

    // The hours are optional: without them only the threshold is blind.
    let mut hourly = Vec::new();
    if let Some(block) = root.get("hourly") {
        let times = block.get("time").and_then(Value::as_array);
        let temps = block.get("temperature_2m").and_then(Value::as_array);
        let (Some(times), Some(temps)) = (times, temps) else {
            return shape("hourly lacks time or temperature_2m");
        };
        if times.len() != temps.len() {
            return shape("hourly time and temperature_2m differ in length");
        }
        for (time, degrees) in times.iter().zip(temps) {
            // The service sends null for an hour it has nothing for.
            if degrees.is_null() {
                continue;
            }
            let time = time
                .as_str()
                .ok_or_else(|| ParseError::Shape("an hourly time is not text".into()))?;
            hourly.push((
                instant(time)?,
                temperature(degrees, "an hourly temperature")?,
            ));
        }
    }
    Ok(Observation {
        at,
        temperature_c,
        // `filter` above bounds it, so this cannot truncate.
        code: u8::try_from(code).unwrap_or(u8::MAX),
        hourly,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let path = format!(
            "{}/tests/fixtures/open-meteo-2026-11-26/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    #[test]
    fn the_request_the_app_makes_reads_in_utc() {
        let o = parse(&fixture("kuala-lumpur-past-3h-gmt.json")).expect("reads");
        assert_eq!(o.at, "2026-11-26T11:15:00Z".parse().expect("t"));
        assert_eq!(o.temperature_c, 26.4);
        assert_eq!(o.code, 3);
        assert_eq!(o.hourly.len(), 4);
        assert_eq!(
            o.hourly[0],
            ("2026-11-26T08:00:00Z".parse().expect("t"), 28.1)
        );
        assert_eq!(o.hourly[3].1, 26.5);
    }

    #[test]
    fn an_answer_in_a_local_zone_is_refused_not_misread() {
        // Kuala Lumpur's local-time capture has an offset of 28800 seconds.
        let error = parse(&fixture("kuala-lumpur-hourly-forecast.json")).expect_err("refused");
        assert!(matches!(error, ParseError::Shape(why) if why.contains("28800")));
    }

    #[test]
    fn london_in_winter_has_a_zero_offset_and_is_read() {
        // Zero offset in a named zone is still UTC: nothing to correct.
        let o = parse(&fixture("london-hourly-forecast.json")).expect("reads");
        assert_eq!((o.temperature_c, o.code), (5.7, 2));
    }

    #[test]
    fn a_refusal_is_named_and_its_reason_kept_for_the_log() {
        for name in [
            "http400-latitude-out-of-range.json",
            "http400-invalid-variable.json",
        ] {
            let error = parse(&fixture(name)).expect_err(name);
            assert!(
                matches!(error, ParseError::Rejected(reason) if !reason.is_empty()),
                "{name}"
            );
        }
    }

    #[test]
    fn an_empty_body_with_status_200_is_not_json() {
        let error = parse(&fixture("http200-no-coordinates-empty-body.txt")).expect_err("empty");
        assert!(matches!(error, ParseError::NotJson(_)));
    }

    #[test]
    fn broken_bodies_fail_without_panicking() {
        let good = String::from_utf8(fixture("kuala-lumpur-past-3h-gmt.json")).expect("utf8");
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("truncated", good.as_bytes()[..good.len() / 2].to_vec()),
            ("html", b"<html><body>502</body></html>".to_vec()),
            ("an array", b"[]".to_vec()),
            ("no current", br#"{"utc_offset_seconds":0}"#.to_vec()),
            (
                "no offset",
                good.replace("\"utc_offset_seconds\":0,", "").into_bytes(),
            ),
            (
                "code 200",
                good.replace("\"weather_code\":3", "\"weather_code\":200")
                    .into_bytes(),
            ),
            (
                "code text",
                good.replace("\"weather_code\":3", "\"weather_code\":\"3\"")
                    .into_bytes(),
            ),
            (
                "null temperature",
                good.replace("26.4", "null").into_bytes(),
            ),
            (
                "absurd temperature",
                good.replace("26.4", "999").into_bytes(),
            ),
            (
                "bad time",
                good.replace("2026-11-26T11:15", "yesterday").into_bytes(),
            ),
            (
                "hours differ",
                good.replace("[28.1,27.8,27.2,26.5]", "[28.1]").into_bytes(),
            ),
        ];
        for (what, body) in cases {
            assert!(parse(&body).is_err(), "{what} should be refused");
        }
    }

    #[test]
    fn a_missing_hour_is_left_out_and_the_rest_read() {
        let good = String::from_utf8(fixture("kuala-lumpur-past-3h-gmt.json")).expect("utf8");
        let o = parse(good.replace("27.8", "null").as_bytes()).expect("reads");
        assert_eq!(o.hourly.len(), 3);
    }

    #[test]
    fn an_answer_without_hours_still_gives_the_current_weather() {
        let body = br#"{"utc_offset_seconds":0,"current":{"time":"2026-11-26T11:15","temperature_2m":20.0,"weather_code":0}}"#;
        let o = parse(body).expect("reads");
        assert!(o.hourly.is_empty());
        assert_eq!(o.code, 0);
    }

    #[test]
    fn extra_fields_are_ignored() {
        let body = br#"{"utc_offset_seconds":0,"surprise":{"a":1},"current":{"time":"2026-11-26T11:15","temperature_2m":20.0,"weather_code":0,"new_thing":5}}"#;
        assert!(parse(body).is_ok());
    }

    #[test]
    fn an_endless_answer_is_refused_by_size() {
        assert_eq!(
            parse(&vec![b' '; MAX_BODY + 1]),
            Err(ParseError::TooLarge(MAX_BODY + 1))
        );
    }
}
