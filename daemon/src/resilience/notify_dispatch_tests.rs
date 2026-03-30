use super::*;

#[test]
fn channel_result_tracks_failure() {
    let r = ChannelResult {
        channel: "ntfy".to_string(),
        success: false,
        error: Some("connection refused".to_string()),
        duration_ms: 10,
    };
    assert!(!r.success);
    assert_eq!(r.error.as_deref(), Some("connection refused"));
}

#[test]
fn channel_result_tracks_success() {
    let r = ChannelResult {
        channel: "telegram".to_string(),
        success: true,
        error: None,
        duration_ms: 5,
    };
    assert!(r.success);
    assert!(r.error.is_none());
}

#[test]
fn channel_result_serializes() {
    let r = ChannelResult {
        channel: "ntfy".to_string(),
        success: false,
        error: Some("timeout".to_string()),
        duration_ms: 42,
    };
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json["channel"], "ntfy");
    assert_eq!(json["success"], false);
    assert_eq!(json["error"], "timeout");
    assert_eq!(json["duration_ms"], 42);
}

#[tokio::test]
async fn dispatch_returns_per_channel_results() {
    let channels = vec![ChannelConfig::MacOS];
    let msg = NotifyMessage {
        title: "Dispatch test".to_string(),
        message: "verify per-channel results".to_string(),
        severity: NotifySeverity::Info,
    };
    let results = dispatch(&channels, &msg).await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].channel, "macos");
    assert!(results[0].duration_ms <= u64::MAX);
    if !results[0].success {
        assert!(results[0].error.is_some(), "failed channel must include error details");
    }
}

#[tokio::test]
async fn dispatch_empty_channels_returns_empty() {
    let msg = NotifyMessage {
        title: "No channels".to_string(),
        message: "nothing".to_string(),
        severity: NotifySeverity::Info,
    };
    let results = dispatch(&[], &msg).await;
    assert!(results.is_empty());
}
