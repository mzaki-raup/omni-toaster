//! Where the user is (FEAT-0056, ADR-021, `docs/11-prayer-and-azan.md` §6).
//!
//! Three ways to know a place — an IP lookup the user consented to, a city
//! search, typed coordinates — and the one rule that decides which to use. Every
//! place is rounded to two decimals (about a kilometre) the moment it arrives,
//! before it is stored or sent anywhere, and no IP address is kept: the lookup's
//! answer carries one and [`Located`] has nowhere to put it.
//!
//! Saving is `ot-store`'s job and the consent prompt is the UI's; this crate
//! decides what to ask next ([`next_step`]) and does the two network calls.

use jiff::{SignedDuration, Timestamp};
use serde_json::Value;
use ureq::Agent;

use crate::fetch::build_agent;
use crate::zones;

const IP_ENDPOINT: &str = "https://ipapi.co/json/";
const SEARCH_ENDPOINT: &str = "https://geocoding-api.open-meteo.com/v1/search";

/// Every Malaysian place has this zone: UTC+8 all year, in Sabah and Sarawak too.
pub const MALAYSIA_TZ: &str = "Asia/Kuala_Lumpur";

/// A detected place is looked up again once it is this old.
pub const DETECTION_TTL: SignedDuration = SignedDuration::from_hours(24);

/// The largest answer read from either service.
const MAX_BODY: u64 = 64 * 1024;

/// How many cities a search offers.
const SEARCH_RESULTS: usize = 5;

/// The longest search text sent.
const MAX_QUERY_CHARS: usize = 100;

/// A place, with its coordinates rounded.
#[derive(Debug, Clone, PartialEq)]
pub struct Located {
    /// What the user sees and confirms: "Shah Alam, Selangor".
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    /// An IANA zone name, known to the zone database.
    pub tz: String,
    pub country_code: Option<String>,
}

/// Two decimals of a degree, about 1.1 km. Never `-0`.
pub fn round(degrees: f64) -> f64 {
    (degrees * 100.0).round() / 100.0 + 0.0
}

/// Whether the user has agreed to the IP lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consent {
    /// Not asked yet. Nothing is looked up.
    Unset,
    Granted,
    Declined,
}

impl From<Option<bool>> for Consent {
    fn from(answer: Option<bool>) -> Self {
        match answer {
            None => Self::Unset,
            Some(true) => Self::Granted,
            Some(false) => Self::Declined,
        }
    }
}

/// How a saved place was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Ip,
    Manual,
}

/// What to do about the location, from what is saved and what is allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The saved place stands.
    UseSaved,
    /// Ask for consent; no lookup until it is given.
    AskConsent,
    /// Look the place up.
    Lookup,
    /// Nothing is saved and nothing may be looked up: the user enters a place.
    EnterManually,
}

/// The rule (spec §6): a place the user chose is never re-detected; a detected
/// one is looked up again after a day, and only with consent; with nothing saved
/// the user is asked once, and a refusal leads to manual entry.
pub fn next_step(consent: Consent, saved: Option<(Origin, Timestamp)>, now: Timestamp) -> Step {
    match (saved, consent) {
        (Some((Origin::Manual, _)), _) => Step::UseSaved,
        (Some((Origin::Ip, at)), Consent::Granted) => {
            if now.duration_since(at) < DETECTION_TTL {
                Step::UseSaved
            } else {
                Step::Lookup
            }
        }
        // Consent withdrawn or never given while a detection is saved: keep it,
        // but never look again.
        (Some((Origin::Ip, _)), _) => Step::UseSaved,
        (None, Consent::Unset) => Step::AskConsent,
        (None, Consent::Granted) => Step::Lookup,
        (None, Consent::Declined) => Step::EnterManually,
    }
}

/// Why a lookup or a search produced no place.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LookupError {
    /// The IP lookup was asked for without the user's consent. No request was
    /// made.
    #[error("the user has not agreed to an IP lookup")]
    NoConsent,
    #[error("the search text is empty or too long")]
    BadQuery,
    #[error("the request failed: {0}")]
    Network(String),
    #[error("the service answered HTTP {0}")]
    Http(u16),
    #[error("the response is over the size limit")]
    TooLarge,
    #[error("the response is not JSON: {0}")]
    NotJson(String),
    /// The service said no: a rate limit, for one.
    #[error("the service refused: {0}")]
    Service(String),
    #[error("the response is not the shape the service sends: {0}")]
    Shape(String),
}

/// The two services' client.
#[derive(Clone)]
pub struct Lookup {
    agent: Agent,
    ip_url: String,
    search_url: String,
}

impl Lookup {
    /// HTTPS only, ten seconds, `OmniToaster/<version>` (ADR-019).
    pub fn new() -> Self {
        Self::with(IP_ENDPOINT, SEARCH_ENDPOINT, true)
    }

    fn with(ip_url: &str, search_url: &str, https_only: bool) -> Self {
        Self {
            agent: build_agent(https_only),
            ip_url: ip_url.to_string(),
            search_url: search_url.to_string(),
        }
    }

    /// Where the PC is, from its IP address. **Needs the user's consent and
    /// makes no request without it.** Blocks, so it runs on a worker thread.
    pub fn detect(&self, consent: Consent) -> Result<Located, LookupError> {
        if consent != Consent::Granted {
            return Err(LookupError::NoConsent);
        }
        let body = self.get(self.agent.get(&self.ip_url))?;
        parse_ip_answer(&body)
    }

    /// Cities matching `text`, in `language` (`en` or `ms`), at most five. The
    /// text typed is sent to Open-Meteo, and only when the user searches.
    pub fn search(&self, text: &str, language: &str) -> Result<Vec<Located>, LookupError> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > MAX_QUERY_CHARS {
            return Err(LookupError::BadQuery);
        }
        let request = self
            .agent
            .get(&self.search_url)
            .query("name", text)
            .query("count", SEARCH_RESULTS.to_string())
            .query("language", language)
            .query("format", "json");
        parse_search_answer(&self.get(request)?)
    }

    fn get(
        &self,
        request: ureq::RequestBuilder<ureq::typestate::WithoutBody>,
    ) -> Result<Vec<u8>, LookupError> {
        let mut response = request
            .call()
            .map_err(|error| LookupError::Network(error.to_string()))?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .with_config()
            .limit(MAX_BODY)
            .read_to_vec()
            .map_err(|error| match error {
                ureq::Error::BodyExceedsLimit(_) => LookupError::TooLarge,
                other => LookupError::Network(other.to_string()),
            });
        // A 429 from the IP service carries its reason in the body.
        match (status, body) {
            (200, body) => body,
            (_, Ok(body)) if serde_json::from_slice::<Value>(&body).is_ok() => Ok(body),
            (status, _) => Err(LookupError::Http(status)),
        }
    }
}

impl Default for Lookup {
    fn default() -> Self {
        Self::new()
    }
}

fn json(body: &[u8]) -> Result<Value, LookupError> {
    serde_json::from_slice(body).map_err(|error| LookupError::NotJson(error.to_string()))
}

fn text<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value
        .get(field)?
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

fn degrees(value: &Value, field: &str, limit: f64) -> Result<f64, LookupError> {
    let number = value
        .get(field)
        .and_then(Value::as_f64)
        .filter(|n| n.is_finite() && n.abs() <= limit)
        .ok_or_else(|| LookupError::Shape(format!("{field} is missing or out of range")))?;
    Ok(round(number))
}

fn zone(value: &Value) -> Result<String, LookupError> {
    let name = text(value, "timezone")
        .ok_or_else(|| LookupError::Shape("timezone is missing".to_string()))?;
    jiff::tz::TimeZone::get(name)
        .map_err(|_| LookupError::Shape(format!("{name:?} is not a time zone")))?;
    Ok(name.to_string())
}

fn country(value: &Value) -> Option<String> {
    text(value, "country_code")
        .filter(|code| code.len() == 2 && code.bytes().all(|b| b.is_ascii_alphabetic()))
        .map(str::to_ascii_uppercase)
}

/// ipapi.co's answer (ADR-021). It carries the IP and much besides, none of
/// which is read.
fn parse_ip_answer(body: &[u8]) -> Result<Located, LookupError> {
    let value = json(body)?;
    if value.get("error").and_then(Value::as_bool) == Some(true) {
        let reason = text(&value, "reason").unwrap_or("no reason given");
        return Err(LookupError::Service(reason.to_string()));
    }
    let name = match (text(&value, "city"), text(&value, "region")) {
        (Some(city), Some(region)) if city != region => format!("{city}, {region}"),
        (Some(city), _) => city.to_string(),
        (None, Some(region)) => region.to_string(),
        (None, None) => text(&value, "country_name")
            .ok_or_else(|| LookupError::Shape("no place name".to_string()))?
            .to_string(),
    };
    Ok(Located {
        name,
        latitude: degrees(&value, "latitude", 90.0)?,
        longitude: degrees(&value, "longitude", 180.0)?,
        tz: zone(&value)?,
        country_code: country(&value),
    })
}

/// Open-Meteo geocoding's answer: no `results` key at all means no match.
fn parse_search_answer(body: &[u8]) -> Result<Vec<Located>, LookupError> {
    let value = json(body)?;
    let Some(results) = value.get("results") else {
        return Ok(Vec::new());
    };
    let results = results
        .as_array()
        .ok_or_else(|| LookupError::Shape("results is not a list".to_string()))?;
    results
        .iter()
        .take(SEARCH_RESULTS)
        .map(|result| {
            let name = text(result, "name")
                .ok_or_else(|| LookupError::Shape("a result has no name".to_string()))?;
            let name = match text(result, "admin1") {
                Some(region) if region != name => format!("{name}, {region}"),
                _ => name.to_string(),
            };
            Ok(Located {
                name,
                latitude: degrees(result, "latitude", 90.0)?,
                longitude: degrees(result, "longitude", 180.0)?,
                tz: zone(result)?,
                country_code: country(result),
            })
        })
        .collect()
}

/// Why typed coordinates were refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoordinateError {
    #[error("expected a latitude and a longitude, like 3.14, 101.69")]
    Format,
    #[error("latitude must be between -90 and 90, longitude between -180 and 180")]
    Range,
    #[error("{0:?} is not a time zone")]
    TimeZone(String),
}

/// A place from typed coordinates — "3.14, 101.69", with a comma, semicolon or
/// space between them. Inside Malaysia the zone is [`MALAYSIA_TZ`]; elsewhere
/// there is no free offline way from coordinates to a zone, so it is
/// `system_tz`, the PC's own, and the user confirms it (ADR-021).
pub fn from_coordinates(input: &str, system_tz: &str) -> Result<Located, CoordinateError> {
    let mut parts = input
        .split([',', ';', ' '])
        .filter(|part| !part.trim().is_empty());
    let (Some(lat), Some(lon), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(CoordinateError::Format);
    };
    let parse = |part: &str| {
        part.trim()
            .parse::<f64>()
            .map_err(|_| CoordinateError::Format)
    };
    let (latitude, longitude) = (parse(lat)?, parse(lon)?);
    if !latitude.is_finite()
        || !longitude.is_finite()
        || latitude.abs() > 90.0
        || longitude.abs() > 180.0
    {
        return Err(CoordinateError::Range);
    }
    let (latitude, longitude) = (round(latitude), round(longitude));
    let (tz, country_code) = if zones::in_malaysia(latitude, longitude, None) {
        (MALAYSIA_TZ.to_string(), Some("MY".to_string()))
    } else {
        jiff::tz::TimeZone::get(system_tz)
            .map_err(|_| CoordinateError::TimeZone(system_tz.to_string()))?;
        (system_tz.to_string(), None)
    };
    Ok(Located {
        name: format!("{latitude}, {longitude}"),
        latitude,
        longitude,
        tz,
        country_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testserver::serve;
    use std::time::Duration;

    // Shapes as the two services send them (probed 2026-11-26), with invented
    // values: the real answer carries the caller's own IP and place.
    const IP_ANSWER: &str = r#"{"ip":"203.0.113.9","version":"IPv4","city":"Shah Alam",
        "region":"Selangor","region_code":"10","country":"MY","country_name":"Malaysia",
        "country_code":"MY","continent_code":"AS","in_eu":false,"postal":"40000",
        "latitude":3.0851234,"longitude":101.5328765,"timezone":"Asia/Kuala_Lumpur",
        "utc_offset":"+0800","asn":"AS0","org":"Example ISP"}"#;

    const SEARCH_ANSWER: &str = r#"{"results":[
        {"id":1,"name":"Shah Alam","latitude":3.08507,"longitude":101.53281,"elevation":30.0,
         "feature_code":"PPLA","country_code":"MY","admin1_id":1,"timezone":"Asia/Kuala_Lumpur",
         "population":481654,"country":"Malaysia","admin1":"Selangor"},
        {"id":2,"name":"Alam","latitude":-6.2,"longitude":106.8,"timezone":"Asia/Jakarta",
         "country_code":"ID"}],"generationtime_ms":0.5}"#;

    fn at(text: &str) -> Timestamp {
        text.parse().expect("an instant")
    }

    fn client(base: &str) -> Lookup {
        Lookup::with(&format!("{base}?ip"), base, false)
    }

    #[test]
    fn coordinates_are_rounded_to_two_decimals_and_never_minus_zero() {
        assert_eq!(round(3.0851234), 3.09);
        assert_eq!(round(101.5328765), 101.53);
        assert_eq!(round(-33.8688), -33.87);
        assert_eq!(round(-0.001).to_bits(), 0.0_f64.to_bits(), "no -0");
    }

    #[test]
    fn nothing_is_looked_up_without_consent_and_a_manual_place_always_stands() {
        let now = at("2026-11-26T12:00:00Z");
        let long_ago = at("2026-10-01T00:00:00Z");
        let a_while_ago = at("2026-11-25T12:00:01Z");
        let a_day_ago = at("2026-11-25T12:00:00Z");

        // Nothing saved: ask once, look up only after yes, enter by hand after no.
        assert_eq!(next_step(Consent::Unset, None, now), Step::AskConsent);
        assert_eq!(next_step(Consent::Granted, None, now), Step::Lookup);
        assert_eq!(next_step(Consent::Declined, None, now), Step::EnterManually);

        // A detection is cached for a day.
        let ip = |at| Some((Origin::Ip, at));
        assert_eq!(
            next_step(Consent::Granted, ip(a_while_ago), now),
            Step::UseSaved
        );
        assert_eq!(
            next_step(Consent::Granted, ip(a_day_ago), now),
            Step::Lookup
        );
        assert_eq!(next_step(Consent::Granted, ip(long_ago), now), Step::Lookup);

        // Consent withdrawn: the saved place stays and is never looked up again.
        for consent in [Consent::Declined, Consent::Unset] {
            assert_eq!(next_step(consent, ip(long_ago), now), Step::UseSaved);
        }

        // A place the user chose overrides everything, however old.
        for consent in [Consent::Unset, Consent::Granted, Consent::Declined] {
            assert_eq!(
                next_step(consent, Some((Origin::Manual, long_ago)), now),
                Step::UseSaved
            );
        }
        assert_eq!(Consent::from(None), Consent::Unset);
        assert_eq!(Consent::from(Some(true)), Consent::Granted);
        assert_eq!(Consent::from(Some(false)), Consent::Declined);
    }

    #[test]
    fn detect_without_consent_makes_no_request() {
        let (base, requests) = serve("200 OK", IP_ANSWER.as_bytes().to_vec(), Duration::ZERO);
        for consent in [Consent::Unset, Consent::Declined] {
            assert_eq!(client(&base).detect(consent), Err(LookupError::NoConsent));
        }
        assert!(
            requests.recv_timeout(Duration::from_millis(300)).is_err(),
            "no request may leave without consent"
        );
    }

    #[test]
    fn detect_reads_the_place_rounds_it_and_keeps_no_ip() {
        let (base, requests) = serve("200 OK", IP_ANSWER.as_bytes().to_vec(), Duration::ZERO);
        let found = client(&base).detect(Consent::Granted).expect("a place");
        assert_eq!(
            found,
            Located {
                name: "Shah Alam, Selangor".to_string(),
                latitude: 3.09,
                longitude: 101.53,
                tz: "Asia/Kuala_Lumpur".to_string(),
                country_code: Some("MY".to_string()),
            }
        );
        assert!(
            !format!("{found:?}").contains("203.0.113"),
            "the IP is not kept"
        );
        let request = requests
            .recv_timeout(Duration::from_secs(10))
            .expect("seen");
        assert!(request.starts_with("GET /index.php?ip"), "{request}");
        assert!(
            request.to_ascii_lowercase().contains(&format!(
                "user-agent: omnitoaster/{}",
                env!("CARGO_PKG_VERSION")
            )),
            "{request}"
        );
    }

    #[test]
    fn a_bad_ip_answer_is_named_not_swallowed() {
        let ask = |status: &'static str, body: &str| {
            let (base, _) = serve(status, body.as_bytes().to_vec(), Duration::ZERO);
            client(&base).detect(Consent::Granted)
        };
        assert_eq!(
            ask(
                "429 Too Many Requests",
                r#"{"error":true,"reason":"RateLimited"}"#
            ),
            Err(LookupError::Service("RateLimited".to_string()))
        );
        assert_eq!(
            ask("500 Internal Server Error", "<html>"),
            Err(LookupError::Http(500))
        );
        assert!(matches!(
            ask("200 OK", "<html>"),
            Err(LookupError::NotJson(_))
        ));
        assert!(matches!(
            ask("200 OK", r#"{"city":"X"}"#),
            Err(LookupError::Shape(_))
        ));
        assert!(matches!(
            ask(
                "200 OK",
                r#"{"city":"X","latitude":91.0,"longitude":0.0,"timezone":"UTC"}"#
            ),
            Err(LookupError::Shape(_))
        ));
        assert!(matches!(
            ask(
                "200 OK",
                r#"{"city":"X","latitude":1.0,"longitude":0.0,"timezone":"Mars/Base"}"#
            ),
            Err(LookupError::Shape(_))
        ));
        let (base, _) = serve("200 OK", vec![b' '; 200_000], Duration::ZERO);
        assert_eq!(
            client(&base).detect(Consent::Granted),
            Err(LookupError::TooLarge)
        );
    }

    #[test]
    fn a_city_search_sends_the_text_encoded_and_reads_rounded_places() {
        let (base, requests) = serve("200 OK", SEARCH_ANSWER.as_bytes().to_vec(), Duration::ZERO);
        let found = client(&base)
            .search("  Shah Alam & co ", "ms")
            .expect("results");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "Shah Alam, Selangor");
        assert_eq!((found[0].latitude, found[0].longitude), (3.09, 101.53));
        assert_eq!(found[0].tz, "Asia/Kuala_Lumpur");
        assert_eq!(found[1].name, "Alam", "no region, no comma");
        assert_eq!(found[1].country_code.as_deref(), Some("ID"));

        let request = requests
            .recv_timeout(Duration::from_secs(10))
            .expect("seen");
        let line = request.lines().next().expect("a request line");
        assert!(
            line.contains("name=Shah%20Alam%20%26%20co"),
            "encoded and trimmed: {line}"
        );
        assert!(
            line.contains("language=ms") && line.contains("count=5"),
            "{line}"
        );
    }

    #[test]
    fn a_search_with_no_match_is_empty_and_a_bad_search_is_refused_before_sending() {
        let (base, _) = serve(
            "200 OK",
            br#"{"generationtime_ms":0.2}"#.to_vec(),
            Duration::ZERO,
        );
        assert_eq!(client(&base).search("Nowhereville", "en"), Ok(Vec::new()));

        let (base, requests) = serve("200 OK", SEARCH_ANSWER.as_bytes().to_vec(), Duration::ZERO);
        let lookup = client(&base);
        for bad in ["", "   ", &"x".repeat(101)] {
            assert_eq!(lookup.search(bad, "en"), Err(LookupError::BadQuery));
        }
        assert!(requests.recv_timeout(Duration::from_millis(300)).is_err());
    }

    /// The live check against both real services, over the real TLS stack:
    /// `cargo test -p ot-prayer -- --ignored`. The IP answer is the caller's own
    /// place, so this asserts its shape and prints no value. Ignored, not
    /// skipped on a connection error, so an unreachable service is a failure
    /// you chose to run.
    #[test]
    #[ignore = "needs ipapi.co and Open-Meteo"]
    fn a_live_lookup_and_a_live_search() {
        let lookup = Lookup::new();

        let places = lookup.search("Shah Alam", "en").expect("a search");
        let first = places.first().expect("Shah Alam is found");
        assert_eq!(first.country_code.as_deref(), Some("MY"));
        assert_eq!(first.tz, MALAYSIA_TZ);

        let here = lookup.detect(Consent::Granted).expect("a lookup");
        assert!(!here.name.is_empty());
        assert_eq!(here.latitude, round(here.latitude), "already rounded");
        assert_eq!(here.longitude, round(here.longitude), "already rounded");
    }

    #[test]
    fn typed_coordinates_inside_malaysia_get_the_malaysian_zone() {
        for text in [
            "3.0851, 101.5328",
            "3.0851 101.5328",
            "3.0851;101.5328",
            " 3.0851,101.5328 ",
        ] {
            let found = from_coordinates(text, "Europe/London").expect(text);
            assert_eq!((found.latitude, found.longitude), (3.09, 101.53), "{text}");
            assert_eq!(found.tz, MALAYSIA_TZ);
            assert_eq!(found.country_code.as_deref(), Some("MY"));
            assert_eq!(found.name, "3.09, 101.53");
        }
    }

    #[test]
    fn typed_coordinates_abroad_use_the_pcs_zone_and_bad_ones_are_refused() {
        let found = from_coordinates("51.5074, -0.1278", "Europe/London").expect("London");
        assert_eq!(
            found.tz, "Europe/London",
            "the PC's zone, for the user to confirm"
        );
        assert_eq!(found.country_code, None);

        for (bad, error) in [
            ("", CoordinateError::Format),
            ("3.1", CoordinateError::Format),
            ("3.1, 101.6, 5", CoordinateError::Format),
            ("north, east", CoordinateError::Format),
            ("91, 0", CoordinateError::Range),
            ("0, 181", CoordinateError::Range),
            ("NaN, 0", CoordinateError::Range),
        ] {
            assert_eq!(from_coordinates(bad, "UTC"), Err(error), "{bad:?}");
        }
        assert_eq!(
            from_coordinates("51.5, -0.1", "Mars/Base"),
            Err(CoordinateError::TimeZone("Mars/Base".to_string()))
        );
        // Inside Malaysia the PC's zone is not consulted, so a bad one is no obstacle.
        assert!(from_coordinates("3.1, 101.6", "Mars/Base").is_ok());
    }
}
