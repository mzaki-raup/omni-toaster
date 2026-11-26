//! The e-Solat client (FEAT-0052, ADR-018, ADR-019).
//!
//! One blocking attempt on a worker thread: [`Client::fetch`] asks for a zone's
//! times over a date range and hands the body to [`jakim::parse`]. It never
//! sleeps, retries or decides when to run again — the scheduler owns time — so
//! the schedule is the pure functions [`next_after_success`] and
//! [`next_after_failure`], which the scheduler calls with an outcome.
//!
//! **Cancel.** Blocking `ureq` cannot interrupt a read in progress, so
//! [`Cancel`] is checked before the request goes out and again before an
//! outcome is delivered, and the ten-second timeout bounds the wait in
//! between. A cancelled [`spawn`] delivers nothing at all, and it never has a
//! path to the cache: the thread only sends over a channel.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use jiff::ToSpan;
use jiff::civil::Date;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::jakim::{self, MAX_BODY, ParseError, PrayerDay, RejectedDay, Response};

const ENDPOINT: &str = "https://www.e-solat.gov.my/index.php";

/// Connect, send and read together may take this long, and no longer.
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// Waits after the first, second and third failure in a row.
pub const RETRY: [Duration; 3] = [
    Duration::from_secs(60),
    Duration::from_secs(5 * 60),
    Duration::from_secs(15 * 60),
];

/// The regular check: weekly.
pub const WEEKLY: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// While the year's end leaves a gap, or JAKIM has not published the new year.
pub const DAILY: Duration = Duration::from_secs(24 * 60 * 60);

/// Why an attempt produced no times.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FetchError {
    #[error("{0:?} is not a JAKIM zone code")]
    BadZone(String),
    /// DNS, connect, TLS, timeout or a broken connection.
    #[error("the request failed: {0}")]
    Network(String),
    #[error("e-Solat answered HTTP {0}")]
    Http(u16),
    #[error("the response is over the {MAX_BODY}-byte limit")]
    TooLarge,
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// `NO_RECORD!`: an unknown zone or a year not yet published.
    #[error("e-Solat has no times for that zone and range")]
    NoRecord,
    #[error("the fetch was cancelled")]
    Cancelled,
}

/// A flag a caller sets to stop a fetch. Clones share it.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// An inclusive range of days.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Date,
    pub end: Date,
}

/// Today to the end of next month. The range is not cut at 31 December here:
/// e-Solat stops a range at the end of a published year by itself, and
/// [`Fetched::cut_short`] shows when it did (ADR-018).
pub fn range(today: Date) -> Range {
    let end = today
        .first_of_month()
        .checked_add(1.month())
        .map_or_else(|_| today.last_of_month(), |next| next.last_of_month());
    Range { start: today, end }
}

/// A zone code is five letters and digits, and it goes into a URL, so nothing
/// else is let through.
pub fn valid_zone(zone: &str) -> bool {
    zone.len() == 5 && zone.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// A request, before it is sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    /// `application/x-www-form-urlencoded`: the dates go in a POST body,
    /// because the same values in the query string give HTTP 500.
    pub body: String,
}

fn request_at(base: &str, zone: &str, range: Range) -> Result<Request, FetchError> {
    if !valid_zone(zone) {
        return Err(FetchError::BadZone(zone.to_string()));
    }
    Ok(Request {
        url: format!(
            "{base}?r=esolatApi/takwimsolat&zone={}&period=duration",
            zone.to_ascii_uppercase()
        ),
        body: format!("datestart={}&dateend={}", range.start, range.end),
    })
}

/// The request for a zone and a range.
pub fn request(zone: &str, range: Range) -> Result<Request, FetchError> {
    request_at(ENDPOINT, zone, range)
}

/// What an attempt read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub zone: String,
    pub days: Vec<PrayerDay>,
    /// Days set aside whole, with the reason; the rest still read.
    pub rejected: Vec<RejectedDay>,
    pub requested: Range,
}

impl Fetched {
    /// Whether the answer stops before the range asked for — the year's end,
    /// when the next year is not published.
    pub fn cut_short(&self) -> bool {
        self.days
            .iter()
            .map(|day| day.date)
            .max()
            .is_none_or(|last| last < self.requested.end)
    }
}

/// When to look again after a good answer: weekly, or daily while the answer
/// stopped short of the range.
pub fn next_after_success(fetched: &Fetched) -> Duration {
    if fetched.cut_short() { DAILY } else { WEEKLY }
}

/// When to try again after a failure, `failures` being the run of failures in
/// a row including this one. `None` means do not retry: the caller cancelled,
/// or the zone code can never work.
pub fn next_after_failure(error: &FetchError, failures: u32) -> Option<Duration> {
    match error {
        FetchError::Cancelled | FetchError::BadZone(_) => None,
        FetchError::NoRecord => Some(DAILY),
        _ => Some(
            usize::try_from(failures.saturating_sub(1))
                .ok()
                .and_then(|n| RETRY.get(n))
                .copied()
                .unwrap_or(WEEKLY),
        ),
    }
}

/// The HTTP client: one agent, reused.
#[derive(Clone)]
pub struct Client {
    agent: ureq::Agent,
    base: String,
}

impl Client {
    /// The client for e-Solat: HTTPS only, ten seconds, `OmniToaster/<version>`.
    pub fn new() -> Self {
        Self::with(ENDPOINT, true)
    }

    fn with(base: &str, https_only: bool) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .https_only(https_only)
            .timeout_global(Some(TIMEOUT))
            .user_agent(format!("OmniToaster/{}", env!("CARGO_PKG_VERSION")))
            // Status codes are read here, so a 500 with a JSON content type
            // and an HTML body is ours to name, not an opaque error.
            .http_status_as_error(false)
            // ureq 3 never picks these up by itself: its default is rustls,
            // which is not built, and with native-tls it panics on the default
            // root list rather than falling back to the Windows store.
            .tls_config(
                TlsConfig::builder()
                    .provider(TlsProvider::NativeTls)
                    .root_certs(RootCerts::PlatformVerifier)
                    .build(),
            )
            .build()
            .into();
        Self {
            agent,
            base: base.to_string(),
        }
    }

    /// One attempt. Blocks for at most [`TIMEOUT`] plus the read.
    pub fn fetch(&self, zone: &str, range: Range, cancel: &Cancel) -> Result<Fetched, FetchError> {
        let request = request_at(&self.base, zone, range)?;
        if cancel.is_cancelled() {
            return Err(FetchError::Cancelled);
        }
        let body = self.send(&request)?;
        if cancel.is_cancelled() {
            return Err(FetchError::Cancelled);
        }
        match jakim::parse(&body)? {
            Response::NoRecord => Err(FetchError::NoRecord),
            Response::Days {
                zone: answered,
                days,
                rejected,
            } => {
                if !answered.eq_ignore_ascii_case(zone) {
                    return Err(ParseError::Shape(format!(
                        "asked for {zone}, e-Solat answered for {answered}"
                    ))
                    .into());
                }
                for day in &rejected {
                    tracing::warn!(zone, date = %day.date, reason = %day.reason, "a prayer day was set aside");
                }
                Ok(Fetched {
                    zone: answered,
                    days,
                    rejected,
                    requested: range,
                })
            }
        }
    }

    fn send(&self, request: &Request) -> Result<Vec<u8>, FetchError> {
        let mut response = self
            .agent
            .post(&request.url)
            .content_type("application/x-www-form-urlencoded")
            .send(request.body.as_str())
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

/// Run one attempt on its own thread and send the outcome to `reply`. A
/// cancelled attempt sends nothing, so a closed window leaves no late result.
pub fn spawn(
    client: Client,
    zone: String,
    range: Range,
    cancel: Cancel,
    reply: mpsc::Sender<Result<Fetched, FetchError>>,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("ot-prayer-fetch".into())
        .spawn(move || {
            let outcome = client.fetch(&zone, range, &cancel);
            if cancel.is_cancelled() {
                return;
            }
            match &outcome {
                Ok(fetched) => tracing::info!(
                    zone = %fetched.zone,
                    days = fetched.days.len(),
                    rejected = fetched.rejected.len(),
                    cut_short = fetched.cut_short(),
                    "fetched prayer times from e-Solat"
                ),
                Err(error) => tracing::warn!(%zone, %error, "the e-Solat fetch failed"),
            }
            // The receiver may be gone; nobody is waiting, which is fine.
            let _ = reply.send(outcome);
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    const SAMPLE: &str = include_str!("../tests/fixtures/esolat-2026-11-20/SGR01-month.json");

    fn date(y: i16, m: i8, d: i8) -> Date {
        Date::new(y, m, d).expect("a real date")
    }

    fn span(start: Date, end: Date) -> Range {
        Range { start, end }
    }

    /// Whether `raw` holds a request's headers and as many body bytes as its
    /// `Content-Length` promised.
    fn request_is_whole(raw: &[u8]) -> bool {
        let text = String::from_utf8_lossy(raw);
        let Some((head, body)) = text.split_once("\r\n\r\n") else {
            return false;
        };
        let promised = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        body.len() >= promised
    }

    /// A one-connection HTTP server on localhost: it waits `delay`, answers
    /// with `status` and `body`, and sends the raw request it read to the
    /// returned receiver. Plain HTTP, so only a test client (HTTPS not forced)
    /// can reach it.
    fn serve(
        status: &'static str,
        body: Vec<u8>,
        delay: Duration,
    ) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!("http://{}/index.php", listener.local_addr().expect("addr"));
        let (seen, requests) = mpsc::channel();
        std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut raw = Vec::new();
            let mut chunk = [0u8; 1024];
            // Read until the headers and the whole form body are in: closing
            // a socket with unread bytes resets the connection.
            while !request_is_whole(&raw) {
                match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => raw.extend_from_slice(&chunk[..n]),
                }
            }
            let _ = seen.send(String::from_utf8_lossy(&raw).into_owned());
            std::thread::sleep(delay);
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let mut answer = head.into_bytes();
            answer.extend_from_slice(&body);
            let _ = stream.write_all(&answer);
            // Stay open until the client closes: closing first, or shutting
            // down the write side, resets the connection under it on Windows.
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            while matches!(stream.read(&mut chunk), Ok(n) if n > 0) {}
        });
        (base, requests)
    }

    fn local(base: &str) -> Client {
        Client::with(base, false)
    }

    #[test]
    fn the_range_runs_to_the_end_of_next_month() {
        assert_eq!(
            range(date(2026, 11, 26)),
            span(date(2026, 11, 26), date(2026, 12, 31))
        );
        // Next month is in the next year: asked for in full, and e-Solat
        // stops it at 31 December by itself.
        assert_eq!(
            range(date(2026, 12, 15)),
            span(date(2026, 12, 15), date(2027, 1, 31))
        );
        assert_eq!(
            range(date(2026, 1, 31)),
            span(date(2026, 1, 31), date(2026, 2, 28))
        );
        assert_eq!(
            range(date(2028, 1, 30)),
            span(date(2028, 1, 30), date(2028, 2, 29))
        );
    }

    #[test]
    fn the_request_is_a_post_body_for_the_zone_and_range() {
        let request =
            request("sgr01", span(date(2026, 11, 26), date(2026, 12, 31))).expect("a valid zone");
        assert_eq!(
            request.url,
            "https://www.e-solat.gov.my/index.php?r=esolatApi/takwimsolat&zone=SGR01&period=duration"
        );
        assert_eq!(request.body, "datestart=2026-11-26&dateend=2026-12-31");
    }

    #[test]
    fn a_zone_that_is_not_a_code_never_reaches_a_url() {
        let range = span(date(2026, 11, 26), date(2026, 12, 31));
        for bad in ["", "SGR1", "SGR011", "SGR01&x=1", "SG R1", "SGR0\u{e9}"] {
            assert_eq!(
                request(bad, range),
                Err(FetchError::BadZone(bad.to_string())),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn the_backoff_is_one_five_fifteen_then_the_next_check() {
        let network = FetchError::Network("down".into());
        let waits: Vec<_> = (1..=5)
            .map(|n| next_after_failure(&network, n).expect("retried"))
            .collect();
        assert_eq!(
            waits,
            [
                Duration::from_secs(60),
                Duration::from_secs(300),
                Duration::from_secs(900),
                WEEKLY,
                WEEKLY
            ]
        );
        assert_eq!(
            next_after_failure(&FetchError::Http(500), 1),
            Some(RETRY[0])
        );
        assert_eq!(next_after_failure(&FetchError::NoRecord, 1), Some(DAILY));
        assert_eq!(next_after_failure(&FetchError::Cancelled, 1), None);
        assert_eq!(
            next_after_failure(&FetchError::BadZone("x".into()), 1),
            None
        );
    }

    #[test]
    fn a_range_cut_short_is_looked_at_daily_and_a_full_one_weekly() {
        let (base, _) = serve("200 OK", SAMPLE.as_bytes().to_vec(), Duration::ZERO);
        let client = local(&base);
        let asked = range(date(2026, 11, 20));
        let fetched = client
            .fetch("SGR01", asked, &Cancel::new())
            .expect("the sample reads");
        // The sample ends on 30 November: short of 31 December.
        assert!(fetched.cut_short());
        assert_eq!(next_after_success(&fetched), DAILY);

        let whole = Fetched {
            requested: span(date(2026, 11, 20), date(2026, 11, 30)),
            ..fetched
        };
        assert!(!whole.cut_short());
        assert_eq!(next_after_success(&whole), WEEKLY);
    }

    #[test]
    fn an_attempt_posts_the_form_with_the_user_agent_and_reads_the_days() {
        let (base, requests) = serve("200 OK", SAMPLE.as_bytes().to_vec(), Duration::ZERO);
        let fetched = local(&base)
            .fetch(
                "SGR01",
                span(date(2026, 11, 20), date(2026, 11, 30)),
                &Cancel::new(),
            )
            .expect("the sample reads");
        assert_eq!(fetched.zone, "SGR01");
        assert_eq!(fetched.days.len(), 30);

        let raw = requests.recv_timeout(TIMEOUT).expect("the server saw it");
        let lower = raw.to_ascii_lowercase();
        assert!(
            raw.starts_with("POST /index.php?r=esolatApi/takwimsolat&zone=SGR01&period=duration"),
            "{raw}"
        );
        assert!(
            lower.contains(&format!(
                "user-agent: omnitoaster/{}",
                env!("CARGO_PKG_VERSION")
            )),
            "{raw}"
        );
        assert!(
            lower.contains("content-type: application/x-www-form-urlencoded"),
            "{raw}"
        );
        assert!(
            raw.ends_with("datestart=2026-11-20&dateend=2026-11-30"),
            "{raw}"
        );
    }

    #[test]
    fn what_goes_wrong_is_named_not_swallowed() {
        let range = span(date(2026, 11, 20), date(2026, 11, 30));

        let (base, _) = serve(
            "500 Internal Server Error",
            b"<html>".to_vec(),
            Duration::ZERO,
        );
        assert_eq!(
            local(&base).fetch("SGR01", range, &Cancel::new()),
            Err(FetchError::Http(500))
        );

        let (base, _) = serve("200 OK", b"<html>oops</html>".to_vec(), Duration::ZERO);
        assert!(matches!(
            local(&base).fetch("SGR01", range, &Cancel::new()),
            Err(FetchError::Parse(ParseError::NotJson(_)))
        ));

        let (base, _) = serve(
            "200 OK",
            br#"{"status":"NO_RECORD!","prayerTime":{"data":["none"]}}"#.to_vec(),
            Duration::ZERO,
        );
        assert_eq!(
            local(&base).fetch("SGR01", range, &Cancel::new()),
            Err(FetchError::NoRecord)
        );

        // An answer for another zone is not the answer asked for.
        let (base, _) = serve("200 OK", SAMPLE.as_bytes().to_vec(), Duration::ZERO);
        assert!(matches!(
            local(&base).fetch("WLY01", range, &Cancel::new()),
            Err(FetchError::Parse(ParseError::Shape(_)))
        ));

        // Nothing is listening: a network error, not a hang.
        let closed = TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!("http://{}/index.php", closed.local_addr().expect("addr"));
        drop(closed);
        assert!(matches!(
            local(&base).fetch("SGR01", range, &Cancel::new()),
            Err(FetchError::Network(_))
        ));
    }

    #[test]
    fn a_body_over_the_cap_stops_at_the_cap() {
        let (base, _) = serve("200 OK", vec![b' '; MAX_BODY * 2], Duration::ZERO);
        assert_eq!(
            local(&base).fetch(
                "SGR01",
                span(date(2026, 11, 20), date(2026, 11, 30)),
                &Cancel::new()
            ),
            Err(FetchError::TooLarge)
        );
    }

    #[test]
    fn production_refuses_plain_http() {
        let (base, requests) = serve("200 OK", SAMPLE.as_bytes().to_vec(), Duration::ZERO);
        let strict = Client::with(&base, true);
        assert!(matches!(
            strict.fetch(
                "SGR01",
                span(date(2026, 11, 20), date(2026, 11, 30)),
                &Cancel::new()
            ),
            Err(FetchError::Network(_))
        ));
        assert!(
            requests.recv_timeout(Duration::from_millis(300)).is_err(),
            "no request may leave over plain http"
        );
    }

    #[test]
    fn a_fetch_cancelled_before_it_starts_sends_nothing() {
        let (base, requests) = serve("200 OK", SAMPLE.as_bytes().to_vec(), Duration::ZERO);
        let cancel = Cancel::new();
        cancel.cancel();
        assert_eq!(
            local(&base).fetch(
                "SGR01",
                span(date(2026, 11, 20), date(2026, 11, 30)),
                &cancel
            ),
            Err(FetchError::Cancelled)
        );
        assert!(
            requests.recv_timeout(Duration::from_millis(300)).is_err(),
            "a cancelled fetch must not reach the server"
        );
    }

    #[test]
    fn a_fetch_cancelled_in_flight_delivers_nothing() {
        // The server holds its answer back; the cancel lands meanwhile.
        let (base, requests) = serve(
            "200 OK",
            SAMPLE.as_bytes().to_vec(),
            Duration::from_millis(600),
        );
        let (reply, outcomes) = mpsc::channel();
        let cancel = Cancel::new();
        let worker = spawn(
            local(&base),
            "SGR01".to_string(),
            span(date(2026, 11, 20), date(2026, 11, 30)),
            cancel.clone(),
            reply,
        )
        .expect("a thread");
        requests
            .recv_timeout(TIMEOUT)
            .expect("the request went out");
        cancel.cancel();
        worker.join().expect("the worker ends");
        assert!(
            outcomes.try_recv().is_err(),
            "a cancelled attempt must deliver no result"
        );
    }

    #[test]
    fn an_attempt_that_is_not_cancelled_delivers_its_outcome() {
        let (base, _) = serve("200 OK", SAMPLE.as_bytes().to_vec(), Duration::ZERO);
        let (reply, outcomes) = mpsc::channel();
        spawn(
            local(&base),
            "SGR01".to_string(),
            span(date(2026, 11, 20), date(2026, 11, 30)),
            Cancel::new(),
            reply,
        )
        .expect("a thread");
        let fetched = outcomes
            .recv_timeout(TIMEOUT)
            .expect("an outcome")
            .expect("the sample reads");
        assert_eq!(fetched.days.len(), 30);
    }

    /// The live check against the real service, over the real TLS stack. It
    /// does not run by default: `cargo test -p ot-prayer -- --ignored`. It is
    /// ignored, not skipped on a connection error, so an unreachable e-Solat
    /// is a failure you chose to run, never a pass that proved nothing.
    #[test]
    #[ignore = "needs e-Solat"]
    fn a_live_fetch_for_three_zones() {
        let client = Client::new();
        // Malaysia's date, whatever zone the PC is set to.
        let today = jiff::Timestamp::now()
            .to_zoned_datetime(jiff::tz::TimeZone::fixed(jiff::tz::offset(8)))
            .date();
        for zone in ["SGR01", "WLY01", "JHR01"] {
            let asked = range(today);
            let fetched = client
                .fetch(zone, asked, &Cancel::new())
                .unwrap_or_else(|error| panic!("{zone}: {error}"));
            assert_eq!(fetched.zone, zone);
            assert!(!fetched.days.is_empty(), "{zone} has days");
            assert!(
                fetched.rejected.is_empty(),
                "{zone}: {:?}",
                fetched.rejected
            );
            assert_eq!(fetched.days[0].date, today, "{zone} starts today");
        }
    }
}
