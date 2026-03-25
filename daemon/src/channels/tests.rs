// Why: Plan 725 T1-02 — tests for ChannelAdapter trait, ChannelMessage, ChannelHealth, ChannelError.
use super::*;
use serde_json::json;

#[test]
fn channel_message_serialization_roundtrip() {
    let msg = ChannelMessage {
        id: "msg-001".into(),
        source_channel: "slack".into(),
        content: "Convergio build passed".into(),
        reply_to: Some("msg-000".into()),
        metadata: json!({"thread_ts": "1711234567.000100"}),
        timestamp: chrono::Utc::now(),
    };
    let serialized = serde_json::to_string(&msg).expect("serialize");
    let deserialized: ChannelMessage = serde_json::from_str(&serialized).expect("deserialize");
    assert_eq!(deserialized.id, "msg-001");
    assert_eq!(deserialized.source_channel, "slack");
    assert_eq!(deserialized.content, "Convergio build passed");
    assert_eq!(deserialized.reply_to, Some("msg-000".into()));
    assert_eq!(deserialized.metadata["thread_ts"], "1711234567.000100");
}

#[test]
fn channel_message_without_reply_to() {
    let msg = ChannelMessage {
        id: "msg-002".into(),
        source_channel: "discord".into(),
        content: "Deployment complete".into(),
        reply_to: None,
        metadata: json!({}),
        timestamp: chrono::Utc::now(),
    };
    let serialized = serde_json::to_string(&msg).expect("serialize");
    let deserialized: ChannelMessage = serde_json::from_str(&serialized).expect("deserialize");
    assert!(deserialized.reply_to.is_none());
}

#[test]
fn channel_health_serialization() {
    let health = ChannelHealth {
        connected: true,
        last_message_at: Some(chrono::Utc::now()),
        error_count: 0,
        channel_name: "slack".into(),
    };
    let serialized = serde_json::to_string(&health).expect("serialize");
    let deserialized: ChannelHealth = serde_json::from_str(&serialized).expect("deserialize");
    assert!(deserialized.connected);
    assert_eq!(deserialized.error_count, 0);
    assert_eq!(deserialized.channel_name, "slack");
}

#[test]
fn channel_health_disconnected() {
    let health = ChannelHealth {
        connected: false,
        last_message_at: None,
        error_count: 5,
        channel_name: "discord".into(),
    };
    assert!(!health.connected);
    assert!(health.last_message_at.is_none());
    assert_eq!(health.error_count, 5);
}

#[test]
fn channel_error_display_connection_failed() {
    let err = ChannelError::ConnectionFailed("timeout after 30s".into());
    assert_eq!(err.to_string(), "connection failed: timeout after 30s");
}

#[test]
fn channel_error_display_rate_limited() {
    let err = ChannelError::RateLimited {
        retry_after_secs: 60,
    };
    assert_eq!(err.to_string(), "rate limited: retry after 60s");
}

#[test]
fn channel_error_display_auth_failed() {
    let err = ChannelError::AuthFailed("invalid token".into());
    assert_eq!(err.to_string(), "authentication failed: invalid token");
}

#[test]
fn channel_error_display_other() {
    let err = ChannelError::Other("unexpected format".into());
    assert_eq!(err.to_string(), "channel error: unexpected format");
}

/// Verify the trait is object-safe by constructing a mock and boxing it.
struct MockAdapter {
    adapter_name: String,
    connected: bool,
}

impl ChannelAdapter for MockAdapter {
    fn connect<'a>(&'a mut self) -> AsyncChannelResult<'a, ()> {
        Box::pin(async {
            self.connected = true;
            Ok(())
        })
    }

    fn send<'a>(&'a self, _msg: &'a ChannelMessage) -> AsyncChannelResult<'a, ()> {
        Box::pin(async { Ok(()) })
    }

    fn disconnect<'a>(&'a mut self) -> AsyncChannelResult<'a, ()> {
        Box::pin(async {
            self.connected = false;
            Ok(())
        })
    }

    fn health<'a>(&'a self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ChannelHealth> + Send + 'a>> {
        Box::pin(async {
            ChannelHealth {
                connected: self.connected,
                last_message_at: None,
                error_count: 0,
                channel_name: self.adapter_name.clone(),
            }
        })
    }

    fn name(&self) -> &str {
        &self.adapter_name
    }
}

#[tokio::test]
async fn mock_adapter_connect_send_disconnect() {
    let mut adapter = MockAdapter {
        adapter_name: "test-channel".into(),
        connected: false,
    };
    assert!(!adapter.health().await.connected);

    adapter.connect().await.expect("connect");
    assert!(adapter.health().await.connected);

    let msg = ChannelMessage {
        id: "msg-100".into(),
        source_channel: "test-channel".into(),
        content: "Integration verified".into(),
        reply_to: None,
        metadata: json!({}),
        timestamp: chrono::Utc::now(),
    };
    adapter.send(&msg).await.expect("send");

    adapter.disconnect().await.expect("disconnect");
    assert!(!adapter.health().await.connected);
}

#[test]
fn trait_is_object_safe() {
    // Compile-time verification: ChannelAdapter can be used as dyn trait object
    fn _accepts_dyn(_adapter: &dyn ChannelAdapter) {}
    fn _accepts_boxed(_adapter: Box<dyn ChannelAdapter>) {}
}

#[test]
fn adapter_name_matches() {
    let adapter = MockAdapter {
        adapter_name: "slack-workspace".into(),
        connected: false,
    };
    assert_eq!(adapter.name(), "slack-workspace");
}
