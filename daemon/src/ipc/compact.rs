//! MessagePack compact serialization for IPC payloads.
//!
//! Provides binary-efficient encode/decode using rmp-serde.
//! Content-Type negotiation: Accept: application/msgpack → MessagePack,
//! Accept: application/json (default) → JSON. Backwards-compatible.

use serde::de::DeserializeOwned;
use serde::Serialize;

/// Serializes a value to MessagePack bytes.
///
/// Produces a compact binary payload — typically 20-40 % smaller than JSON
/// for structs with many named fields.
pub fn serialize_compact<T: Serialize>(value: &T) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    rmp_serde::to_vec_named(value)
}

/// Deserializes a value from MessagePack bytes.
pub fn deserialize_compact<T: DeserializeOwned>(data: &[u8]) -> Result<T, rmp_serde::decode::Error> {
    rmp_serde::from_slice(data)
}

/// Returns true when the Accept header requests MessagePack.
///
/// Defaults to JSON for all other values (backwards-compatible).
pub fn prefers_msgpack(accept: &str) -> bool {
    accept.contains("application/msgpack")
}

#[cfg(test)]
mod inline_tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Peer {
        id: String,
        latency_ms: u32,
    }

    #[test]
    fn prefers_msgpack_detects_header() {
        assert!(prefers_msgpack("application/msgpack"));
        assert!(prefers_msgpack("application/msgpack, */*;q=0.1"));
        assert!(!prefers_msgpack("application/json"));
        assert!(!prefers_msgpack("*/*"));
        assert!(!prefers_msgpack(""));
    }

    #[test]
    fn roundtrip_peer() {
        let p = Peer { id: "node-42".into(), latency_ms: 12 };
        let bytes = serialize_compact(&p).unwrap();
        let decoded: Peer = deserialize_compact(&bytes).unwrap();
        assert_eq!(p, decoded);
    }
}
