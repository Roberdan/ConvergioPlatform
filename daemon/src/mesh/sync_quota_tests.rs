// Tests for PeerQuota rate limiting and MeshSyncFrame serialization.
// Covers: PeerQuota::new, with_limit, reserve, release, overflow,
//         Default trait, and frame serde roundtrips for all variants.

use super::*;

// === PeerQuota tests ===

#[test]
fn peer_quota_new_starts_at_zero() {
    let q = PeerQuota::new();
    assert_eq!(q.pending_bytes(), 0);
}

#[test]
fn peer_quota_default_equals_new() {
    let q1 = PeerQuota::new();
    let q2 = PeerQuota::default();
    assert_eq!(q1, q2);
}

#[test]
fn peer_quota_with_limit_starts_at_zero() {
    let q = PeerQuota::with_limit(1024);
    assert_eq!(q.pending_bytes(), 0);
}

#[test]
fn peer_quota_reserve_increases_pending() {
    let mut q = PeerQuota::with_limit(1000);
    q.reserve(500).expect("within limit");
    assert_eq!(q.pending_bytes(), 500);
}

#[test]
fn peer_quota_reserve_exactly_at_limit_succeeds() {
    let mut q = PeerQuota::with_limit(100);
    q.reserve(100).expect("exactly at limit");
    assert_eq!(q.pending_bytes(), 100);
}

#[test]
fn peer_quota_reserve_over_limit_fails() {
    let mut q = PeerQuota::with_limit(100);
    let err = q.reserve(101).expect_err("over limit");
    assert!(err.to_string().contains("mesh peer pending bytes exceeded"));
}

#[test]
fn peer_quota_reserve_cumulative_over_limit_fails() {
    let mut q = PeerQuota::with_limit(100);
    q.reserve(60).expect("first reserve ok");
    let err = q.reserve(50).expect_err("cumulative > limit");
    assert!(err.to_string().contains("exceeded"));
    // pending should remain at 60 (failed reserve doesn't change state)
    assert_eq!(q.pending_bytes(), 60);
}

#[test]
fn peer_quota_release_decreases_pending() {
    let mut q = PeerQuota::with_limit(1000);
    q.reserve(500).expect("reserve");
    q.release(200);
    assert_eq!(q.pending_bytes(), 300);
}

#[test]
fn peer_quota_release_saturates_at_zero() {
    let mut q = PeerQuota::with_limit(1000);
    q.reserve(100).expect("reserve");
    q.release(200); // release more than pending
    assert_eq!(q.pending_bytes(), 0, "saturating_sub should clamp to 0");
}

#[test]
fn peer_quota_release_then_reserve_succeeds() {
    let mut q = PeerQuota::with_limit(100);
    q.reserve(100).expect("fill to limit");
    q.release(100);
    q.reserve(50).expect("re-reserve after release");
    assert_eq!(q.pending_bytes(), 50);
}

#[test]
fn peer_quota_reserve_overflow_u64_fails() {
    let mut q = PeerQuota::with_limit(usize::MAX);
    q.reserve(usize::MAX).expect("max reserve");
    let err = q.reserve(1).expect_err("overflow");
    assert!(err.to_string().contains("overflow"));
}

// === MeshSyncFrame serde roundtrips (all variants) ===

#[test]
fn msgpack_roundtrip_heartbeat_frame() {
    let frame = MeshSyncFrame::Heartbeat {
        node: "node-alpha".into(),
        ts: 1_700_000_000_000,
    };
    let bytes = rmp_serde::to_vec_named(&frame).expect("encode");
    let decoded: MeshSyncFrame = rmp_serde::from_slice(&bytes).expect("decode");
    assert_eq!(decoded, frame);
}

#[test]
fn msgpack_roundtrip_ack_frame() {
    let frame = MeshSyncFrame::Ack {
        node: "responder".into(),
        applied: 42,
        latency_ms: 15,
        last_db_version: 999,
    };
    let bytes = rmp_serde::to_vec_named(&frame).expect("encode");
    let decoded: MeshSyncFrame = rmp_serde::from_slice(&bytes).expect("decode");
    assert_eq!(decoded, frame);
}

#[test]
fn msgpack_roundtrip_delta_with_null_val() {
    let frame = MeshSyncFrame::Delta {
        node: "n1".into(),
        sent_at_ms: 100,
        last_db_version: 5,
        changes: vec![DeltaChange {
            table_name: "tasks".into(),
            pk: b"id=1".to_vec(),
            cid: "deleted_col".into(),
            val: None, // NULL value — important edge case
            col_version: 1,
            db_version: 5,
            site_id: b"n1".to_vec(),
            cl: 1,
            seq: 1,
        }],
    };
    let bytes = rmp_serde::to_vec_named(&frame).expect("encode");
    let decoded: MeshSyncFrame = rmp_serde::from_slice(&bytes).expect("decode");
    assert_eq!(decoded, frame);
}

#[test]
fn msgpack_roundtrip_delta_empty_changes() {
    let frame = MeshSyncFrame::Delta {
        node: "sender".into(),
        sent_at_ms: 50,
        last_db_version: 0,
        changes: vec![],
    };
    let bytes = rmp_serde::to_vec_named(&frame).expect("encode");
    let decoded: MeshSyncFrame = rmp_serde::from_slice(&bytes).expect("decode");
    assert_eq!(decoded, frame);
}

#[test]
fn delta_change_equality_checks_all_fields() {
    let a = DeltaChange {
        table_name: "tasks".into(),
        pk: b"id=1".to_vec(),
        cid: "title".into(),
        val: Some("hello".into()),
        col_version: 1,
        db_version: 1,
        site_id: b"n1".to_vec(),
        cl: 1,
        seq: 1,
    };
    let mut b = a.clone();
    assert_eq!(a, b);
    b.cid = "status".into();
    assert_ne!(a, b);
}

#[test]
fn framed_mesh_sync_frame_stores_payload_len() {
    let frame = MeshSyncFrame::Heartbeat {
        node: "x".into(),
        ts: 0,
    };
    let payload = rmp_serde::to_vec_named(&frame).expect("encode");
    let framed = FramedMeshSyncFrame {
        frame: frame.clone(),
        payload_len: payload.len() as u32,
    };
    assert_eq!(framed.frame, frame);
    assert!(framed.payload_len > 0);
}

#[test]
fn max_frame_bytes_is_16mb() {
    assert_eq!(MAX_FRAME_BYTES, 16 * 1024 * 1024);
}

#[test]
fn max_pending_peer_bytes_is_32mb() {
    assert_eq!(MAX_PENDING_PEER_BYTES, 32 * 1024 * 1024);
}

#[test]
fn max_peer_name_len_is_256() {
    assert_eq!(MAX_PEER_NAME_LEN, 256);
}
