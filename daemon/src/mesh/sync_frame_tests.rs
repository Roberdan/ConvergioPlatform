use super::*;
use tokio::io::{duplex, AsyncWriteExt};

#[tokio::test]
async fn read_frame_with_quota_decodes_frame_within_limit() {
    let frame = MeshSyncFrame::Heartbeat {
        node: "peer-a".to_string(),
        ts: 123,
    };
    let (mut writer, mut reader) = duplex(1024);
    write_frame(&mut writer, &frame).await.expect("write frame");
    writer.shutdown().await.expect("shutdown writer");

    let mut quota = PeerQuota::new();
    let framed = read_frame_with_quota(&mut reader, &mut quota)
        .await
        .expect("read frame")
        .expect("frame");
    assert_eq!(framed.frame, frame);
    assert!(
        quota.pending_bytes() > 0,
        "payload remains reserved until released"
    );
    quota.release(framed.payload_len as usize);
    assert_eq!(quota.pending_bytes(), 0);
}

#[tokio::test]
async fn read_frame_with_quota_rejects_oversized_frame() {
    let oversized = MAX_FRAME_BYTES + 1;
    let (mut writer, mut reader) = duplex(64);
    writer
        .write_all(&oversized.to_be_bytes())
        .await
        .expect("write length");
    writer.shutdown().await.expect("shutdown writer");

    let mut quota = PeerQuota::new();
    let err = read_frame_with_quota(&mut reader, &mut quota)
        .await
        .expect_err("oversized frame must be rejected");
    assert!(err.contains("mesh frame exceeds limit"));
    assert_eq!(
        quota.pending_bytes(),
        0,
        "oversized frame must not reserve bytes"
    );
}

#[tokio::test]
async fn peer_quota_blocks_multi_frame_flood_until_release() {
    let frame_a = MeshSyncFrame::Heartbeat {
        node: "peer-a".to_string(),
        ts: 1,
    };
    let frame_b = MeshSyncFrame::Heartbeat {
        node: "peer-b".to_string(),
        ts: 2,
    };
    let payload_a = rmp_serde::to_vec_named(&frame_a).expect("encode a");
    let payload_b = rmp_serde::to_vec_named(&frame_b).expect("encode b");
    let quota_limit = payload_a.len() + payload_b.len() - 1;

    let (mut writer, mut reader) = duplex(1024);
    writer
        .write_all(&(payload_a.len() as u32).to_be_bytes())
        .await
        .expect("write len a");
    writer.write_all(&payload_a).await.expect("write payload a");
    writer
        .write_all(&(payload_b.len() as u32).to_be_bytes())
        .await
        .expect("write len b");
    writer.write_all(&payload_b).await.expect("write payload b");
    writer.shutdown().await.expect("shutdown writer");

    let mut quota = PeerQuota::with_limit(quota_limit);
    let first = read_frame_with_quota(&mut reader, &mut quota)
        .await
        .expect("read first")
        .expect("first frame");
    assert_eq!(first.frame, frame_a);
    let err = read_frame_with_quota(&mut reader, &mut quota)
        .await
        .expect_err("second frame must be blocked while first is pending");
    assert!(err.contains("mesh peer pending bytes exceeded"));

    quota.release(first.payload_len as usize);
    assert_eq!(
        quota.pending_bytes(),
        0,
        "release should clear pending bytes"
    );
}
