//! Tests for MessagePack compact serialization.
//!
//! TDD: these tests drive the implementation in compact.rs.

use super::compact::{deserialize_compact, serialize_compact};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Sample {
    name: String,
    value: u32,
    flag: bool,
}

#[test]
fn roundtrip_simple_struct() {
    let original = Sample {
        name: "relay-node-1".to_string(),
        value: 42,
        flag: true,
    };
    let bytes = serialize_compact(&original).expect("serialize must succeed");
    let decoded: Sample = deserialize_compact(&bytes).expect("deserialize must succeed");
    assert_eq!(original, decoded);
}

#[test]
fn compact_bytes_smaller_than_json() {
    let original = Sample {
        name: "relay-node-1".to_string(),
        value: 42,
        flag: true,
    };
    let msgpack_bytes = serialize_compact(&original).expect("serialize");
    let json_bytes = serde_json::to_vec(&original).expect("json serialize");
    // MessagePack should be more compact for field-heavy structs
    assert!(
        msgpack_bytes.len() < json_bytes.len(),
        "msgpack ({} bytes) should be smaller than json ({} bytes)",
        msgpack_bytes.len(),
        json_bytes.len()
    );
}

#[test]
fn serialize_returns_non_empty_bytes() {
    let data: Vec<u8> = serialize_compact(&42u32).expect("serialize");
    assert!(!data.is_empty());
}

#[test]
fn deserialize_invalid_bytes_returns_error() {
    let bad_bytes = b"not valid msgpack data !!!";
    let result: Result<Sample, _> = deserialize_compact(bad_bytes);
    assert!(result.is_err(), "invalid bytes should return an error");
}

#[test]
fn roundtrip_nested_struct() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Outer {
        inner: Sample,
        count: i64,
    }
    let original = Outer {
        inner: Sample {
            name: "machine-7".to_string(),
            value: 100,
            flag: false,
        },
        count: -1,
    };
    let bytes = serialize_compact(&original).expect("serialize");
    let decoded: Outer = deserialize_compact(&bytes).expect("deserialize");
    assert_eq!(original, decoded);
}
