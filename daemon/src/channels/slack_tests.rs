// Why: Plan 725 T3-01 — TDD tests for SlackAdapter (ChannelAdapter impl).
// Tests run against a mock HTTP server to avoid real Slack API calls.

use super::*;
use crate::channels::*;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: create adapter pointing at mock server base URL.
fn test_adapter(base_url: &str) -> SlackAdapter {
    SlackAdapter::new_with_base_url(
        "xoxb-test-token-1234567890abcdef".into(),
        "C01ABCDEF01".into(),
        base_url.into(),
    )
}

#[test]
fn adapter_name_returns_slack() {
    let adapter = test_adapter("http://localhost:1");
    assert_eq!(adapter.name(), "slack");
}

#[tokio::test]
async fn connect_validates_token_via_auth_test() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/auth.test"))
        .and(header("Authorization", "Bearer xoxb-test-token-1234567890abcdef"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "url": "https://convergio.slack.com/",
            "team": "Convergio",
            "user": "convergio_bot",
            "team_id": "T01ABCDEF",
            "user_id": "U01ABCDEF",
            "bot_id": "B01ABCDEF"
        })))
        .mount(&server)
        .await;

    let mut adapter = test_adapter(&server.uri());
    adapter.connect().await.expect("connect should succeed");
    assert!(adapter.is_connected());
}

#[tokio::test]
async fn connect_fails_on_empty_token() {
    let adapter_result = SlackAdapter::new("".into(), "C01ABCDEF01".into());
    assert!(adapter_result.is_err());
    let err = adapter_result.unwrap_err().to_string();
    assert!(err.contains("token"), "got: {err}");
}

#[tokio::test]
async fn connect_fails_on_invalid_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/auth.test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": false,
            "error": "invalid_auth"
        })))
        .mount(&server)
        .await;

    let mut adapter = test_adapter(&server.uri());
    let result = adapter.connect().await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("invalid_auth") || err.contains("authentication failed"),
        "got: {err}"
    );
}

#[tokio::test]
async fn send_posts_chat_message() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat.postMessage"))
        .and(header("Authorization", "Bearer xoxb-test-token-1234567890abcdef"))
        .and(body_json(json!({
            "channel": "C01ABCDEF01",
            "text": "Deployment #47 completed successfully"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "channel": "C01ABCDEF01",
            "ts": "1711234567.000100",
            "message": { "text": "Deployment #47 completed successfully" }
        })))
        .mount(&server)
        .await;

    let adapter = test_adapter(&server.uri());
    let msg = ChannelMessage {
        id: "msg-slack-001".into(),
        source_channel: "slack".into(),
        content: "Deployment #47 completed successfully".into(),
        reply_to: None,
        metadata: json!({}),
        timestamp: chrono::Utc::now(),
    };
    adapter.send(&msg).await.expect("send should succeed");
}

#[tokio::test]
async fn send_fails_on_channel_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat.postMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": false,
            "error": "channel_not_found"
        })))
        .mount(&server)
        .await;

    let adapter = test_adapter(&server.uri());
    let msg = ChannelMessage {
        id: "msg-slack-002".into(),
        source_channel: "slack".into(),
        content: "Should fail".into(),
        reply_to: None,
        metadata: json!({}),
        timestamp: chrono::Utc::now(),
    };
    let result = adapter.send(&msg).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("channel_not_found"), "got: {err}");
}

#[tokio::test]
async fn health_reflects_disconnected_state() {
    let adapter = test_adapter("http://localhost:1");
    let health = adapter.health().await;
    assert!(!health.connected);
    assert_eq!(health.channel_name, "slack");
    assert!(health.last_message_at.is_none());
    assert_eq!(health.error_count, 0);
}

#[tokio::test]
async fn disconnect_sets_connected_false() {
    let mut adapter = test_adapter("http://localhost:1");
    // Simulate already connected
    adapter.set_connected_for_test(true);
    assert!(adapter.is_connected());
    adapter.disconnect().await.expect("disconnect should succeed");
    assert!(!adapter.is_connected());
}

#[test]
fn command_parse_status() {
    let cmd = parse_slash_command("/convergio-status");
    assert!(cmd.is_some());
    let (name, arg) = cmd.unwrap();
    assert_eq!(name, "convergio-status");
    assert!(arg.is_none());
}

#[test]
fn command_parse_approve_with_id() {
    let cmd = parse_slash_command("/convergio-approve task-42");
    assert!(cmd.is_some());
    let (name, arg) = cmd.unwrap();
    assert_eq!(name, "convergio-approve");
    assert_eq!(arg.as_deref(), Some("task-42"));
}

#[test]
fn command_parse_unknown_returns_none() {
    let cmd = parse_slash_command("/unknown-command");
    assert!(cmd.is_none());
}

#[test]
fn format_status_response() {
    let resp = format_command_response("convergio-status", None);
    assert!(resp.contains("status") || resp.contains("operational"), "got: {resp}");
}

#[test]
fn format_approve_response_includes_task_id() {
    let resp = format_command_response("convergio-approve", Some("task-99"));
    assert!(resp.contains("task-99"), "got: {resp}");
    assert!(resp.to_lowercase().contains("approv"), "got: {resp}");
}
