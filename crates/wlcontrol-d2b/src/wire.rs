//! Wire framing primitives for the d2bd public socket.
//!
//! The public protocol frames every message as a 4-byte little-endian unsigned
//! length prefix followed by one UTF-8 JSON document, with a 1 MiB cap
//! (`docs/reference/daemon-api.md`).

use d2b_client::{read_frame, write_frame, FrameBounds};
use futures::{executor::block_on, io::Cursor};
use wlcontrol_core::error::{WlError, WlResult};

/// Maximum accepted frame size (1 MiB), matching the daemon.
pub const MAX_FRAME_BYTES: usize = FrameBounds::default_public_daemon().max_len();

/// Encode a JSON payload into a length-prefixed frame.
pub fn encode_frame(json: &[u8]) -> WlResult<Vec<u8>> {
    let mut frame = Cursor::new(Vec::with_capacity(4 + json.len()));
    block_on(write_frame(
        &mut frame,
        json,
        FrameBounds::default_public_daemon(),
    ))
    .map_err(client_error_to_protocol)?;
    Ok(frame.into_inner())
}

/// Decode a length-prefixed frame, returning the JSON payload bytes.
///
/// `frame` must contain the 4-byte prefix followed by exactly the declared
/// number of payload bytes (as delivered by a single `SOCK_SEQPACKET` message).
pub fn decode_frame(frame: &[u8]) -> WlResult<&[u8]> {
    let mut cursor = Cursor::new(frame.to_vec());
    let decoded = block_on(read_frame(
        &mut cursor,
        FrameBounds::default_public_daemon(),
    ))
    .map_err(client_error_to_protocol)?;
    let consumed = usize::try_from(cursor.position())
        .map_err(|_| WlError::Protocol("frame cursor position overflow".to_owned()))?;
    if consumed != frame.len() {
        return Err(WlError::Protocol(format!(
            "frame payload length {} does not match declared {}",
            frame.len().saturating_sub(4),
            decoded.len()
        )));
    }
    Ok(&frame[4..])
}

fn client_error_to_protocol(err: d2b_client::ClientError) -> WlError {
    WlError::Protocol(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_payload() {
        let payload = br#"{"kind":"list"}"#;
        let frame = encode_frame(payload).expect("encode");
        assert_eq!(&frame[0..4], &(payload.len() as u32).to_le_bytes());
        let decoded = decode_frame(&frame).expect("decode");
        assert_eq!(decoded, payload);
    }

    #[test]
    fn rejects_oversized_encode() {
        let big = vec![0u8; MAX_FRAME_BYTES + 1];
        assert!(encode_frame(&big).is_err());
    }

    #[test]
    fn rejects_truncated_frame() {
        assert!(decode_frame(&[0, 0]).is_err());
    }

    #[test]
    fn rejects_length_mismatch() {
        // Declares 10 bytes but carries 2.
        let frame = [10u8, 0, 0, 0, b'h', b'i'];
        assert!(decode_frame(&frame).is_err());
    }
}
