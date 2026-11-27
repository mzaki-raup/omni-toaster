//! The check (FEAT-0077): ask this copy's forge for the newest release and say
//! whether it is newer, installable and what it is called.
//!
//! One blocking attempt on a worker thread, like the prayer and weather clients:
//! it never sleeps or retries (the service decides when to ask), it checks the
//! cancel flag before the request and before it answers, and it reads at most
//! [`MAX_JSON`](crate::release::MAX_JSON) bytes. It trusts nothing it reads: the
//! answer is an offer for the user to see, and what is installed is decided by the
//! signed manifest (FEAT-0078).

use semver::Version;

use ot_net::Cancel;

use crate::release::{self, MAX_JSON, Release, ReleaseError};
use crate::source::Repo;

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The newest release is this version or older.
    UpToDate { latest: Version },
    /// A newer release that can be installed.
    Available(Release),
    /// A newer release that cannot be: it lacks the manifest, its signature or the
    /// exe, so the user is pointed at its page and told so (`docs/specs/updater.md` §3).
    NotInstallable(Release),
}

/// Why a check produced no answer.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CheckError {
    #[error("the request failed: {0}")]
    Network(String),
    #[error("the forge answered HTTP {0}")]
    Http(u16),
    #[error("the answer is over the {MAX_JSON}-byte limit")]
    TooLarge,
    /// No stable release has been published at all.
    #[error("there is no published release")]
    NoRelease,
    #[error(transparent)]
    Release(#[from] ReleaseError),
    #[error("the check was cancelled")]
    Cancelled,
}

/// The client: one agent, aimed at one repository.
#[derive(Clone)]
pub struct Client {
    agent: ureq::Agent,
    api: String,
}

impl Client {
    /// The client for the repository this copy was built for.
    pub fn new() -> Self {
        Self::for_repo(&crate::source::this_repo())
    }

    pub fn for_repo(repo: &Repo) -> Self {
        Self {
            agent: ot_net::agent(true),
            api: repo.latest_release_api(),
        }
    }

    /// The address that will be asked, for the log: which forge a copy reads is
    /// decided by who built it and is otherwise invisible.
    pub fn api(&self) -> &str {
        &self.api
    }

    #[cfg(test)]
    fn local(api: &str) -> Self {
        Self {
            agent: ot_net::agent(false),
            api: api.to_owned(),
        }
    }

    /// Ask, and compare with `running`. Blocks for at most the agent's timeout.
    pub fn check(&self, running: &Version, cancel: &Cancel) -> Result<Outcome, CheckError> {
        if cancel.is_cancelled() {
            return Err(CheckError::Cancelled);
        }
        tracing::info!(api = %self.api, "checking for a newer release");
        let body = self.get(&self.api)?;
        if cancel.is_cancelled() {
            return Err(CheckError::Cancelled);
        }
        let release = release::parse(&body)?;
        Ok(if !release::is_newer(&release.version, running) {
            Outcome::UpToDate {
                latest: release.version,
            }
        } else if release.installable() {
            Outcome::Available(release)
        } else {
            Outcome::NotInstallable(release)
        })
    }

    fn get(&self, url: &str) -> Result<Vec<u8>, CheckError> {
        let mut response = self
            .agent
            .get(url)
            .header("Accept", "application/json")
            .call()
            .map_err(|error| CheckError::Network(error.to_string()))?;
        match response.status().as_u16() {
            200 => {}
            // Forgejo and GitHub both say 404 when nothing has been published.
            404 => return Err(CheckError::NoRelease),
            other => return Err(CheckError::Http(other)),
        }
        response
            .body_mut()
            .with_config()
            .limit(MAX_JSON as u64)
            .read_to_vec()
            .map_err(|error| match error {
                ureq::Error::BodyExceedsLimit(_) => CheckError::TooLarge,
                other => CheckError::Network(other.to_string()),
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
    use std::time::Duration;

    const FORGEJO: &str = include_str!("../tests/fixtures/releases-2026-11-27/forgejo-latest.json");

    /// A one-connection HTTP server on localhost answering `status` and `body`,
    /// and reporting the request it read.
    fn serve(status: &'static str, body: Vec<u8>) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!(
            "http://{}/api/v1/repos/owner/omni-toaster/releases/latest",
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
            let _ = stream.read(&mut chunk);
        });
        (base, requests)
    }

    fn v(s: &str) -> Version {
        Version::parse(s).expect("semver")
    }

    #[test]
    fn a_newer_installable_release_is_offered_and_the_request_says_it_wants_json() {
        let (api, requests) = serve("200 OK", FORGEJO.as_bytes().to_vec());
        let outcome = Client::local(&api)
            .check(&v("0.17.0"), &Cancel::new())
            .expect("checks");
        let Outcome::Available(release) = outcome else {
            panic!("expected an offer, got {outcome:?}");
        };
        assert_eq!(release.version, v("0.18.0"));
        let request = requests
            .recv_timeout(Duration::from_secs(10))
            .expect("a request");
        assert!(
            request.starts_with("GET /api/v1/repos/owner/omni-toaster/releases/latest"),
            "{request}"
        );
        assert!(
            request
                .to_ascii_lowercase()
                .contains("accept: application/json"),
            "{request}"
        );
        assert!(
            request.contains(&format!("OmniToaster/{}", env!("CARGO_PKG_VERSION"))),
            "{request}"
        );
    }

    #[test]
    fn the_same_or_a_newer_running_version_is_up_to_date() {
        for running in ["0.18.0", "0.19.0", "1.0.0"] {
            let (api, _) = serve("200 OK", FORGEJO.as_bytes().to_vec());
            let outcome = Client::local(&api)
                .check(&v(running), &Cancel::new())
                .expect("checks");
            assert_eq!(
                outcome,
                Outcome::UpToDate {
                    latest: v("0.18.0")
                },
                "running {running}"
            );
        }
    }

    #[test]
    fn a_newer_release_without_its_manifest_is_found_but_not_installable() {
        let body = br#"{"tag_name":"v9.0.0","html_url":"https://h/r","assets":[
            {"name":"OmniToaster.exe","browser_download_url":"https://h/x/OmniToaster.exe","size":5}]}"#;
        let (api, _) = serve("200 OK", body.to_vec());
        let outcome = Client::local(&api)
            .check(&v("0.18.0"), &Cancel::new())
            .expect("checks");
        assert!(matches!(outcome, Outcome::NotInstallable(r) if r.version == v("9.0.0")));
    }

    #[test]
    fn nothing_published_a_server_error_and_a_broken_answer_are_each_named() {
        let (api, _) = serve("404 Not Found", br#"{"message":"Not Found"}"#.to_vec());
        assert_eq!(
            Client::local(&api).check(&v("0.1.0"), &Cancel::new()),
            Err(CheckError::NoRelease)
        );
        let (api, _) = serve("503 Service Unavailable", b"down".to_vec());
        assert_eq!(
            Client::local(&api).check(&v("0.1.0"), &Cancel::new()),
            Err(CheckError::Http(503))
        );
        let (api, _) = serve("200 OK", b"<html>captive portal</html>".to_vec());
        assert!(matches!(
            Client::local(&api).check(&v("0.1.0"), &Cancel::new()),
            Err(CheckError::Release(ReleaseError::NotJson(_)))
        ));
    }

    #[test]
    fn an_endless_answer_is_cut_at_the_cap() {
        let (api, _) = serve("200 OK", vec![b' '; MAX_JSON + 10]);
        assert_eq!(
            Client::local(&api).check(&v("0.1.0"), &Cancel::new()),
            Err(CheckError::TooLarge)
        );
    }

    #[test]
    fn a_cancel_before_the_request_sends_nothing() {
        let (api, requests) = serve("200 OK", FORGEJO.as_bytes().to_vec());
        let cancel = Cancel::new();
        cancel.cancel();
        assert_eq!(
            Client::local(&api).check(&v("0.1.0"), &cancel),
            Err(CheckError::Cancelled)
        );
        assert!(
            requests.recv_timeout(Duration::from_millis(300)).is_err(),
            "nothing reached the server"
        );
    }

    #[test]
    fn an_unreachable_server_is_a_network_error_not_a_panic() {
        let error = Client::local("http://127.0.0.1:9/x")
            .check(&v("0.1.0"), &Cancel::new())
            .expect_err("fails");
        assert!(matches!(error, CheckError::Network(_)), "{error:?}");
    }

    #[test]
    fn the_real_client_is_aimed_at_this_builds_forge_over_https() {
        let client = Client::new();
        assert!(client.api().starts_with("https://"));
        assert!(
            client.api().ends_with("/omni-toaster/releases/latest"),
            "{}",
            client.api()
        );
    }

    /// The live check against both real forges, over the real TLS stack. It does not
    /// run by default: `cargo test -p ot-update -- --ignored`. An unreachable forge is
    /// a failure you chose to run, never a pass that proved nothing.
    #[test]
    #[ignore = "needs git.zacx.net and github.com"]
    fn a_live_check_against_both_forges_finds_an_installable_release() {
        for repo in [
            "https://git.zacx.net/owner/omni-toaster",
            "https://github.com/mzaki-raup/omni-toaster",
        ] {
            let client = Client::for_repo(&Repo::parse(repo).expect("parses"));
            let outcome = client
                .check(&v("0.0.1"), &Cancel::new())
                .unwrap_or_else(|e| panic!("{repo}: {e}"));
            let Outcome::Available(release) = outcome else {
                panic!("{repo}: expected an offer over 0.0.1, got {outcome:?}");
            };
            assert!(release.installable(), "{repo}: {:?}", release.assets);
        }
    }
}
