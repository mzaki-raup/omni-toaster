//! The Open-Meteo client (FEAT-0065, ADR-023).
//!
//! One blocking attempt on a worker thread: [`Client::fetch`] asks for a
//! place's current weather and the last hours and hands the body to
//! [`parse`](crate::observation::parse). It never sleeps, retries or decides when
//! to run again — the service does — so the schedule is the pure function
//! [`next_after_failure`]. Like the prayer client it uses the Windows TLS stack
//! (ADR-019), and it checks [`Cancel`] before the request goes out and again
//! before an outcome is delivered, since blocking `ureq` cannot interrupt a read
//! in progress; the ten-second timeout bounds the wait in between.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::observation::{MAX_BODY, Observation, ParseError, parse};

const ENDPOINT: &str = "https://api.open-meteo.com/v1/forecast";

/// Connect, send and read together may take this long, and no longer.
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// Waits after the first, second and third failure in a row.
pub const RETRY: [Duration; 3] = [
    Duration::from_secs(60),
    Duration::from_secs(5 * 60),
    Duration::from_secs(15 * 60),
];

/// How many hours before now the request asks for: what a temperature
/// threshold looks back over.
pub const HISTORY_HOURS: u32 = 6;

/// Why an attempt produced no observation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FetchError {
    #[error("{0} is not a place on Earth")]
    BadPlace(String),
    /// DNS, connect, TLS, timeout or a broken connection.
    #[error("the request failed: {0}")]
    Network(String),
    #[error("Open-Meteo answered HTTP {0}")]
    Http(u16),
    #[error("the response is over the {MAX_BODY}-byte limit")]
    TooLarge,
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error("the fetch was cancelled")]
    Cancelled,
}

/// A way to stop an attempt that is waiting to start or to deliver.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A coordinate rounded to two decimals — about a kilometre — which is all a
/// forecast needs and all that is sent (`docs/06-security-and-privacy.md`).
pub fn round_coordinate(degrees: f64) -> f64 {
    (degrees * 100.0).round() / 100.0
}

/// The request URL for a place. The coordinates are rounded here, so no caller
/// can send more than that, and they are checked: a latitude past 90 is a
/// mistake of ours, not something to put on the wire.
pub fn request_url(base: &str, latitude: f64, longitude: f64) -> Result<String, FetchError> {
    if !latitude.is_finite() || !(-90.0..=90.0).contains(&latitude) {
        return Err(FetchError::BadPlace(format!("latitude {latitude}")));
    }
    if !longitude.is_finite() || !(-180.0..=180.0).contains(&longitude) {
        return Err(FetchError::BadPlace(format!("longitude {longitude}")));
    }
    Ok(format!(
        "{base}?latitude={:.2}&longitude={:.2}&current=temperature_2m,weather_code\
         &hourly=temperature_2m&past_hours={HISTORY_HOURS}&forecast_hours=1&timezone=GMT",
        round_coordinate(latitude),
        round_coordinate(longitude),
    ))
}

/// How long to wait after the `failures`-th failure in a row, or `None` when
/// waiting would not help (a place that is not on Earth; a cancel).
pub fn next_after_failure(error: &FetchError, failures: u32) -> Option<Duration> {
    match error {
        FetchError::BadPlace(_) | FetchError::Cancelled => None,
        _ => {
            let step = usize::try_from(failures.saturating_sub(1)).unwrap_or(usize::MAX);
            Some(RETRY[step.min(RETRY.len() - 1)])
        }
    }
}

/// The HTTP agent: ten seconds, `OmniToaster/<version>`, Windows TLS with the
/// Windows root store (ADR-019), and status codes left to the caller.
/// `https_only` is off only for the tests' local server.
fn build_agent(https_only: bool) -> ureq::Agent {
    ureq::Agent::config_builder()
        .https_only(https_only)
        .timeout_global(Some(TIMEOUT))
        .user_agent(format!("OmniToaster/{}", env!("CARGO_PKG_VERSION")))
        .http_status_as_error(false)
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

/// The HTTP client: one agent, reused.
#[derive(Clone)]
pub struct Client {
    agent: ureq::Agent,
    base: String,
}

impl Client {
    /// The client for Open-Meteo: HTTPS only, ten seconds, `OmniToaster/<version>`.
    pub fn new() -> Self {
        Self::with(ENDPOINT, true)
    }

    fn with(base: &str, https_only: bool) -> Self {
        Self {
            agent: build_agent(https_only),
            base: base.to_string(),
        }
    }

    /// One attempt. Blocks for at most [`TIMEOUT`] plus the read.
    pub fn fetch(
        &self,
        latitude: f64,
        longitude: f64,
        cancel: &Cancel,
    ) -> Result<Observation, FetchError> {
        let url = request_url(&self.base, latitude, longitude)?;
        if cancel.is_cancelled() {
            return Err(FetchError::Cancelled);
        }
        let body = self.send(&url)?;
        if cancel.is_cancelled() {
            return Err(FetchError::Cancelled);
        }
        Ok(parse(&body)?)
    }

    fn send(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|error| FetchError::Network(error.to_string()))?;
        let status = response.status().as_u16();
        if status != 200 {
            return Err(FetchError::Http(status));
        }
        // The limit applies as the body is read, so an endless answer stops
        // at the cap instead of filling memory before the parser sees it.
        response
            .body_mut()
            .with_config()
            .limit(MAX_BODY as u64)
            .read_to_vec()
            .map_err(|error| match error {
                ureq::Error::BodyExceedsLimit(_) => FetchError::TooLarge,
                other => FetchError::Network(other.to_string()),
            })
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    const SAMPLE: &str =
        include_str!("../tests/fixtures/open-meteo-2026-11-26/kuala-lumpur-past-3h-gmt.json");

    /// A one-connection HTTP server on localhost: it answers with `status` and
    /// `body` and sends the request line it read to the returned receiver.
    fn serve(status: &'static str, body: Vec<u8>) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!(
            "http://{}/v1/forecast",
            listener.local_addr().expect("addr")
        );
        let (seen, requests) = mpsc::channel();
        std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut raw = Vec::new();
            let mut chunk = [0u8; 1024];
            while !raw.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => raw.extend_from_slice(&chunk[..n]),
                }
            }
            let _ = seen.send(String::from_utf8_lossy(&raw).into_owned());
            let mut answer = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .into_bytes();
            answer.extend_from_slice(&body);
            let _ = stream.write_all(&answer);
            let _ = stream.flush();
            // Wait for the client to close, so the answer is never cut by a reset.
            let _ = stream.read(&mut chunk);
        });
        (base, requests)
    }

    fn local(base: &str) -> Client {
        Client::with(base, false)
    }

    #[test]
    fn coordinates_are_rounded_to_two_decimals_before_they_are_sent() {
        assert_eq!(round_coordinate(3.1282952), 3.13);
        assert_eq!(round_coordinate(-0.13078308), -0.13);
        let url = request_url("https://x/f", 3.1282952, 101.68548).expect("url");
        assert!(url.contains("latitude=3.13&longitude=101.69&"), "{url}");
        assert!(url.contains("timezone=GMT") && url.contains("past_hours=6"));
        assert!(!url.contains("3.128"), "a longer coordinate leaked: {url}");
    }

    #[test]
    fn a_place_off_the_earth_is_refused_before_anything_is_sent() {
        for (lat, lon) in [
            (91.0, 0.0),
            (0.0, 181.0),
            (f64::NAN, 0.0),
            (0.0, f64::INFINITY),
        ] {
            assert!(
                matches!(
                    request_url("https://x/f", lat, lon),
                    Err(FetchError::BadPlace(_))
                ),
                "{lat},{lon}"
            );
        }
    }

    #[test]
    fn the_retry_schedule_is_one_five_then_fifteen_minutes() {
        let wait = |n| next_after_failure(&FetchError::Http(503), n);
        assert_eq!(wait(1), Some(Duration::from_secs(60)));
        assert_eq!(wait(2), Some(Duration::from_secs(300)));
        assert_eq!(wait(3), Some(Duration::from_secs(900)));
        assert_eq!(
            wait(9),
            Some(Duration::from_secs(900)),
            "it stays at the last step"
        );
        assert_eq!(
            next_after_failure(&FetchError::BadPlace("x".into()), 1),
            None
        );
        assert_eq!(next_after_failure(&FetchError::Cancelled, 1), None);
    }

    #[test]
    fn a_good_answer_is_read_and_the_request_is_a_plain_get_with_our_agent() {
        let (base, requests) = serve("200 OK", SAMPLE.as_bytes().to_vec());
        let observation = local(&base)
            .fetch(3.15, 101.69, &Cancel::new())
            .expect("reads");
        assert_eq!(observation.code, 3);
        let request = requests.recv_timeout(TIMEOUT).expect("a request");
        assert!(
            request.starts_with("GET /v1/forecast?latitude=3.15&longitude=101.69&"),
            "{request}"
        );
        assert!(
            request.contains(&format!("OmniToaster/{}", env!("CARGO_PKG_VERSION"))),
            "{request}"
        );
    }

    #[test]
    fn a_refusal_status_is_named() {
        let (base, _) = serve(
            "400 Bad Request",
            br#"{"error":true,"reason":"no"}"#.to_vec(),
        );
        let error = local(&base)
            .fetch(3.15, 101.69, &Cancel::new())
            .expect_err("fails");
        assert_eq!(error, FetchError::Http(400));
    }

    #[test]
    fn an_empty_answer_with_status_200_is_a_failure() {
        let (base, _) = serve("200 OK", Vec::new());
        let error = local(&base)
            .fetch(3.15, 101.69, &Cancel::new())
            .expect_err("fails");
        assert!(
            matches!(error, FetchError::Parse(ParseError::NotJson(_))),
            "{error:?}"
        );
    }

    #[test]
    fn an_endless_answer_is_cut_at_the_cap() {
        let (base, _) = serve("200 OK", vec![b' '; MAX_BODY + 10]);
        let error = local(&base)
            .fetch(3.15, 101.69, &Cancel::new())
            .expect_err("fails");
        assert_eq!(error, FetchError::TooLarge);
    }

    #[test]
    fn a_cancel_before_the_request_sends_nothing() {
        let (base, requests) = serve("200 OK", SAMPLE.as_bytes().to_vec());
        let cancel = Cancel::new();
        cancel.cancel();
        assert_eq!(
            local(&base).fetch(3.15, 101.69, &cancel),
            Err(FetchError::Cancelled)
        );
        assert!(
            requests.recv_timeout(Duration::from_millis(300)).is_err(),
            "a cancelled attempt must not reach the server"
        );
    }

    #[test]
    fn an_unreachable_server_is_a_network_error_not_a_panic() {
        // Nothing listens on port 9.
        let error = local("http://127.0.0.1:9/v1/forecast")
            .fetch(3.15, 101.69, &Cancel::new())
            .expect_err("fails");
        assert!(matches!(error, FetchError::Network(_)), "{error:?}");
    }

    /// The live check against the real service, over the real TLS stack. It does
    /// not run by default: `cargo test -p ot-weather -- --ignored`. It is
    /// ignored, not skipped on a connection error, so an unreachable
    /// Open-Meteo is a failure you chose to run, never a pass that proved nothing.
    #[test]
    #[ignore = "needs Open-Meteo"]
    fn a_live_fetch_for_three_places() {
        let client = Client::new();
        for (name, lat, lon) in [
            ("Kuala Lumpur", 3.15, 101.69),
            ("Kuching", 1.55, 110.35),
            ("London", 51.51, -0.13),
        ] {
            let observation = client
                .fetch(lat, lon, &Cancel::new())
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            assert!(observation.code <= 99, "{name}");
            assert!(!observation.hourly.is_empty(), "{name} has hours");
            let age = jiff::Timestamp::now().duration_since(observation.at);
            assert!(
                age.as_secs().abs() < 3 * 3600,
                "{name}: current is {age:?} old"
            );
        }
    }
}
