//! Canonical d2b client adapter boundary.
//!
//! The legacy public-JSON transport is intentionally absent. Live inventory and
//! control remain unavailable until the owning canonical service contracts are
//! content-frozen; this crate does not guess those APIs.

use std::time::Duration;

use wlcontrol_core::{
    error::{WlError, WlResult},
    model::{Connectivity, SocketIntent},
    sources::ReduceInput,
    Config,
};

pub use d2b_client_toolkit::{
    D2B_SOURCE_FINGERPRINT as CLIENT_SOURCE_FINGERPRINT,
    D2B_SOURCE_REVISION as CLIENT_SOURCE_REVISION,
};

const SERVICES_UNAVAILABLE: &str =
    "canonical daemon control services are not available in this source cut";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchOutcome {
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct D2bClient {
    socket_path: String,
    timeout: Duration,
}

impl D2bClient {
    pub fn new(config: &Config) -> Self {
        Self {
            socket_path: config.public_socket.clone(),
            timeout: Duration::from_millis(config.command_timeout_ms),
        }
    }

    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn refresh(&self) -> ReduceInput {
        ReduceInput {
            connectivity: Connectivity::DaemonDown,
            ..Default::default()
        }
    }

    pub fn dispatch(&self, _intent: &SocketIntent) -> WlResult<DispatchOutcome> {
        Err(WlError::D2b(SERVICES_UNAVAILABLE.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binds_the_exact_canonical_source() {
        assert_eq!(
            CLIENT_SOURCE_REVISION,
            "7e94327951d30913a1a6e0e7a47d4a24b462deff"
        );
        assert_eq!(
            CLIENT_SOURCE_FINGERPRINT,
            "0401d1f463d9dad49efd663d1493184e42954f624a029fbfd41c49f0323e5708"
        );
    }

    #[test]
    fn blocked_services_fail_without_a_protocol_fallback() {
        let client = D2bClient::new(&Config::default());
        assert_eq!(client.refresh().connectivity, Connectivity::DaemonDown);
        let error = client
            .dispatch(&SocketIntent::List)
            .expect_err("blocked service must fail closed");
        assert!(matches!(error, WlError::D2b(message) if message == SERVICES_UNAVAILABLE));
    }
}
