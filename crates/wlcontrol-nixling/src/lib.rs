//! Direct nixlingd public-socket client.
//!
//! Owning wave: **Wave 1 — Protocol client agent**. Wave 0 fixes the public
//! API surface (this module's signatures) so the Waybar / GTK / CLI crates can
//! build against a stable seam. The Wave 1 agent implements the real protocol:
//!
//! - connect to the non-abstract `SOCK_SEQPACKET` socket at the configured path;
//! - send the `Hello` negotiation frame and enforce the selected version range;
//! - length-prefix (4-byte little-endian) every JSON request frame;
//! - read bounded responses and map `PublicResponse::Error` /
//!   `MutatingVerbResponse` into typed [`WlError`] values;
//! - translate raw nixling wire JSON into the neutral [`ReduceInput`] fragments.
//!
//! The protocol/transport details live in [`wire`]; high-level intents live on
//! [`NixlingClient`].

use std::time::Duration;

use wlcontrol_core::error::{WlError, WlResult};
use wlcontrol_core::model::{Connectivity, SocketIntent};
use wlcontrol_core::sources::ReduceInput;
use wlcontrol_core::Config;

pub mod wire;

/// Outcome of a single dispatched mutating intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchOutcome {
    /// Human-facing one-line summary suitable for the UI.
    pub summary: String,
}

/// A connected (or connectable) nixlingd public-socket client.
///
/// The client is cheap to construct and does not hold a persistent connection;
/// each call connects, negotiates, performs the request, and closes. This keeps
/// the daemon-down/auth-denied posture observable on every refresh.
#[derive(Debug, Clone)]
pub struct NixlingClient {
    socket_path: String,
    timeout: Duration,
}

impl NixlingClient {
    /// Build a client from user configuration.
    pub fn new(config: &Config) -> Self {
        Self {
            socket_path: config.public_socket.clone(),
            timeout: Duration::from_millis(config.command_timeout_ms),
        }
    }

    /// The configured public-socket path.
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    /// The per-operation timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Collect one full refresh bundle: auth, inventory, per-VM statuses, and
    /// USB claims, translated into neutral [`ReduceInput`] fragments.
    ///
    /// Wave 1 replaces this stub with the real protocol. Until then it reports
    /// a daemon-down posture so the UI degrades safely rather than asserting a
    /// false-healthy state.
    pub fn refresh(&self) -> ReduceInput {
        // TODO(Wave 1 — protocol client): connect, negotiate, query
        // `auth status` / `list` / `status <vm>` / `usb probe`, and populate
        // every fragment. The fake-nixlingd harness (Wave 1 — test agent)
        // exercises this path.
        ReduceInput {
            connectivity: Connectivity::DaemonDown,
            ..Default::default()
        }
    }

    /// Dispatch a single typed socket intent (`vm start`, `usb attach`, ...).
    ///
    /// Wave 1 replaces this stub with the real request/response handling.
    pub fn dispatch(&self, _intent: &SocketIntent) -> WlResult<DispatchOutcome> {
        // TODO(Wave 1 — protocol client): map each intent onto the matching
        // `PublicRequest`, send it framed, and decode the typed response.
        Err(WlError::Protocol(
            "nixling public-socket client not yet implemented (Wave 1)".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_carries_config() {
        let config = Config {
            public_socket: "/tmp/test.sock".into(),
            command_timeout_ms: 1234,
            ..Default::default()
        };
        let client = NixlingClient::new(&config);
        assert_eq!(client.socket_path(), "/tmp/test.sock");
        assert_eq!(client.timeout(), Duration::from_millis(1234));
    }

    #[test]
    fn refresh_stub_reports_daemon_down() {
        let client = NixlingClient::new(&Config::default());
        assert_eq!(client.refresh().connectivity, Connectivity::DaemonDown);
    }
}
