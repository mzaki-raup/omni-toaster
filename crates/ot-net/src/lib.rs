//! What every network service in OmniToaster shares: one way to build the HTTP
//! agent, and one cancel flag (CR-0009, ADR-023's "a third service needs them").
//!
//! The prayer, weather and update services are each a blocking `ureq` call on a
//! worker thread (ADR-002). They differ in what they ask and how they read the
//! answer, never in how they connect, so that lives here once: Windows' own TLS
//! with the Windows certificate store (ADR-019), HTTPS only, a timeout,
//! `User-Agent: OmniToaster/<version>`, and status codes left to the caller.
//!
//! Service crates may not depend on each other (`docs/03-architecture.md`), and
//! this one is not a service: it holds no policy and reads no answer.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

/// Connect, send and read together may take this long, and no longer, for the
/// small answers the prayer and weather services read.
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// A flag a caller sets to stop an attempt that is waiting to start or to
/// deliver. Clones share it. Blocking `ureq` cannot interrupt a read in progress,
/// so services check it before the request goes out and again before an outcome
/// is delivered, and the timeout bounds the wait in between.
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

/// The agent every service call uses, with the ten-second [`TIMEOUT`].
/// `https_only` is off only for a test's local server.
pub fn agent(https_only: bool) -> ureq::Agent {
    agent_with_timeout(https_only, TIMEOUT)
}

/// The same agent with a different overall timeout, for a download that is not
/// a few hundred bytes.
pub fn agent_with_timeout(https_only: bool, timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .https_only(https_only)
        .timeout_global(Some(timeout))
        .user_agent(format!("OmniToaster/{}", env!("CARGO_PKG_VERSION")))
        // Status codes are read by the caller, so a 500 with a JSON content type
        // and an HTML body is theirs to name, not an opaque error.
        .http_status_as_error(false)
        // ureq 3 never picks these up by itself: its default is rustls, which is
        // not built, and with native-tls it panics on the default root list
        // rather than falling back to the Windows store.
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cancel_is_shared_by_its_clones_and_starts_unset() {
        let cancel = Cancel::new();
        let clone = cancel.clone();
        assert!(!cancel.is_cancelled() && !clone.is_cancelled());
        clone.cancel();
        assert!(cancel.is_cancelled(), "the original sees a clone's cancel");
    }

    #[test]
    fn the_agent_builds_with_and_without_https_forced() {
        // Building it proves the TLS configuration is accepted; a request needs a
        // server, which each service's own tests provide.
        let _ = agent(true);
        let _ = agent(false);
        let _ = agent_with_timeout(true, Duration::from_secs(120));
    }
}
