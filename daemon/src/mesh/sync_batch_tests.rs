// Tests for SyncBatchWindow batching/coalescing and frame I/O async roundtrips.

use super::*;

// === SyncBatchWindow tests ===

#[test]
fn batch_window_new_starts_empty() {
    let w = SyncBatchWindow::new(100);
    assert_eq!(w.take_checkpoint(), 0);
    assert!(!w.should_flush(99999));
}

#[test]
fn batch_window_observe_sets_checkpoint() {
    let mut w = SyncBatchWindow::new(100);
    w.observe_change_at(10, 42);
    assert_eq!(w.take_checkpoint(), 42);
}

#[test]
fn batch_window_should_flush_after_window_ms() {
    let mut w = SyncBatchWindow::new(50);
    w.observe_change_at(100, 1);
    assert!(!w.should_flush(149)); // 49ms elapsed
    assert!(w.should_flush(150)); // exactly 50ms
    assert!(w.should_flush(200)); // well past
}

#[test]
fn batch_window_should_flush_false_without_observe() {
    let w = SyncBatchWindow::new(50);
    // No observe_change_at called
    assert!(!w.should_flush(0));
    assert!(!w.should_flush(1000));
}

#[test]
fn batch_window_clear_resets_flush_state() {
    let mut w = SyncBatchWindow::new(50);
    w.observe_change_at(100, 5);
    assert!(w.should_flush(200));
    w.clear();
    // After clear, should_flush returns false (since_ms is None)
    assert!(!w.should_flush(300));
    // But checkpoint is retained
    assert_eq!(w.take_checkpoint(), 5);
}

#[test]
fn batch_window_multiple_observes_keep_first_since() {
    let mut w = SyncBatchWindow::new(100);
    w.observe_change_at(10, 1);
    w.observe_change_at(50, 2);
    w.observe_change_at(80, 3);
    // since_ms should be 10 (first observe), so flush at 110+
    assert!(!w.should_flush(109));
    assert!(w.should_flush(110));
    // checkpoint should be the latest
    assert_eq!(w.take_checkpoint(), 3);
}

#[test]
fn batch_window_zero_ms_flushes_immediately() {
    let mut w = SyncBatchWindow::new(0);
    w.observe_change_at(100, 7);
    assert!(w.should_flush(100)); // 0ms elapsed, matches window_ms=0
}

#[test]
fn batch_window_large_window_ms() {
    let mut w = SyncBatchWindow::new(u64::MAX);
    w.observe_change_at(0, 1);
    // Saturating sub: u64::MAX - 0 = u64::MAX, never >= u64::MAX? Actually it is.
    // 0.saturating_sub(0) = 0, 0 >= u64::MAX? No.
    assert!(!w.should_flush(1_000_000));
}

#[test]
fn batch_window_clear_then_re_observe_works() {
    let mut w = SyncBatchWindow::new(50);
    w.observe_change_at(0, 1);
    w.clear();
    w.observe_change_at(200, 2);
    assert!(!w.should_flush(230)); // 30ms since second observe
    assert!(w.should_flush(250)); // 50ms since second observe
    assert_eq!(w.take_checkpoint(), 2);
}

#[test]
fn current_time_ms_returns_positive_value() {
    let t = current_time_ms();
    // Should be well past year 2000 (946684800000 ms)
    assert!(t > 946_684_800_000, "time should be after year 2000");
}

// === Frame I/O async roundtrip tests ===

#[tokio::test]
async fn write_then_read_frame_roundtrip() {
    let frame = MeshSyncFrame::Heartbeat {
        node: "peer-round".into(),
        ts: 42,
    };
    let (mut writer, mut reader) = tokio::io::duplex(4096);
    write_frame(&mut writer, &frame).await.expect("write");
    drop(writer); // signal EOF
    let decoded = read_frame(&mut reader).await.expect("read").expect("frame");
    assert_eq!(decoded, frame);
}

#[tokio::test]
async fn write_then_read_delta_frame_roundtrip() {
    let frame = MeshSyncFrame::Delta {
        node: "delta-peer".into(),
        sent_at_ms: 999,
        last_db_version: 10,
        changes: vec![
            DeltaChange {
                table_name: "tasks".into(),
                pk: b"pk1".to_vec(),
                cid: "status".into(),
                val: Some("done".into()),
                col_version: 2,
                db_version: 10,
                site_id: b"s1".to_vec(),
                cl: 1,
                seq: 1,
            },
        ],
    };
    let (mut writer, mut reader) = tokio::io::duplex(4096);
    write_frame(&mut writer, &frame).await.expect("write");
    drop(writer);
    let decoded = read_frame(&mut reader).await.expect("read").expect("frame");
    assert_eq!(decoded, frame);
}

#[tokio::test]
async fn read_frame_returns_none_on_eof() {
    let (_writer, mut reader) = tokio::io::duplex(64);
    drop(_writer); // immediate EOF
    let result = read_frame(&mut reader).await.expect("should not error");
    assert!(result.is_none(), "EOF should yield None");
}

#[tokio::test]
async fn write_multiple_frames_then_read_sequentially() {
    let frames = vec![
        MeshSyncFrame::Heartbeat { node: "a".into(), ts: 1 },
        MeshSyncFrame::Ack {
            node: "b".into(),
            applied: 5,
            latency_ms: 10,
            last_db_version: 20,
        },
        MeshSyncFrame::AuthResult {
            ok: true,
            reason: String::new(),
        },
    ];
    let (mut writer, mut reader) = tokio::io::duplex(4096);
    for f in &frames {
        write_frame(&mut writer, f).await.expect("write");
    }
    drop(writer);
    for expected in &frames {
        let decoded = read_frame(&mut reader).await.expect("read").expect("frame");
        assert_eq!(&decoded, expected);
    }
    // Next read should be None (EOF)
    let eof = read_frame(&mut reader).await.expect("eof read");
    assert!(eof.is_none());
}

#[tokio::test]
async fn read_frame_rejects_truncated_payload() {
    use tokio::io::AsyncWriteExt;
    let (mut writer, mut reader) = tokio::io::duplex(256);
    // Write a length header claiming 100 bytes but only write 10
    writer.write_all(&100_u32.to_be_bytes()).await.expect("write len");
    writer.write_all(&[0u8; 10]).await.expect("write partial");
    drop(writer); // EOF after 10 bytes
    let err = read_frame(&mut reader).await.expect_err("should fail on truncated");
    assert!(
        err.to_string().contains("truncated") || err.to_string().contains("mesh frame"),
        "error should mention truncation: {err}"
    );
}
