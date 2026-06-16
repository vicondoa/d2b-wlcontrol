//! Error types shared across nixling-wlcontrol.

use thiserror::Error;

/// Top-level error type for the control surface.
///
/// Owning wave: Wave 0 (integrator) for the variant skeleton; Wave 1 fleet
/// agents may add variants as needed without renaming existing ones.
#[derive(Debug, Error)]
pub enum WlError {
    /// `nixlingd` could not be reached on the public socket.
    #[error("nixlingd is unreachable: {0}")]
    DaemonDown(String),

    /// The public-socket peer rejected the handshake or denied the request.
    #[error("nixlingd denied the request: {0}")]
    Denied(String),

    /// A nixling operation returned a typed error envelope.
    #[error("nixling error: {0}")]
    Nixling(String),

    /// Local configuration could not be loaded or parsed.
    #[error("configuration error: {0}")]
    Config(String),

    /// A wire-protocol framing / serialization failure.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// An operation exceeded its deadline.
    #[error("timed out after {0}")]
    Timeout(String),

    /// I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Convenience result alias.
pub type WlResult<T> = std::result::Result<T, WlError>;
