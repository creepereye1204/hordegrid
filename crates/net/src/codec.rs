//! Wire codec: `[PROTO_VER][postcard(ggrs::Message)]`. Untrusted input → decode never panics.

use ggrs::Message;
use thiserror::Error;

/// Protocol version byte. Bump on any incompatible change (also changes the matchmaking appId).
pub const PROTO_VER: u8 = 1;
/// Hard cap for one datagram (SCTP-fragmentation safe).
pub const MAX_PACKET: usize = 1200;

/// Why a packet was dropped.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
    /// Empty or oversized buffer.
    #[error("bad packet size {0}")]
    Size(usize),
    /// Peer runs another protocol version.
    #[error("protocol version {0} != {PROTO_VER}")]
    Version(u8),
    /// Body failed to deserialize.
    #[error("malformed body")]
    Malformed,
}

/// Encode a GGRS message. Returns `None` if it would exceed [`MAX_PACKET`].
pub fn encode(msg: &Message) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(64);
    out.push(PROTO_VER);
    let body = postcard::to_allocvec(msg).ok()?;
    out.extend_from_slice(&body);
    (out.len() <= MAX_PACKET).then_some(out)
}

/// Decode a datagram.
///
/// # Errors
/// See [`DecodeError`].
pub fn decode(bytes: &[u8]) -> Result<Message, DecodeError> {
    if bytes.is_empty() || bytes.len() > MAX_PACKET {
        return Err(DecodeError::Size(bytes.len()));
    }
    if bytes[0] != PROTO_VER {
        return Err(DecodeError::Version(bytes[0]));
    }
    postcard::from_bytes(&bytes[1..]).map_err(|_| DecodeError::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn when_wrong_version_then_rejected() {
        assert_eq!(decode(&[9, 0, 0]).err(), Some(DecodeError::Version(9)));
        assert_eq!(decode(&[]).err(), Some(DecodeError::Size(0)));
    }

    proptest! {
        /// core-beliefs #4: arbitrary bytes never panic the decoder.
        #[test]
        fn when_fuzzing_decoder_then_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..1300)) {
            let _ = decode(&bytes);
        }
    }
}
