// Why: Plan 725 T2-01 — TDD tests for TelegramAdapter (ChannelAdapter impl).
// Tests run against a mock HTTP server to avoid real Telegram API calls.

use super::*;
use crate::channels::*;
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: create adapter pointing at mock server.
fn test_adapter(base_url: &str) -> TelegramAdapter {
    TelegramAdapter::new_with_base_url(
        "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11".into(),
        Some(42),
        base_url.into(),
    )
}

#[tokio::test]
async fn connect_parses_get_me_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/bot123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11/getMe"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "result": {
                "id": 987654321,
                "is_bot": true,
                "first_name": "ConvergioBot",
                "username": "convergio_bot"
            }
        })))
        .mount(&server)
        .await;

    let mut adapter = test_adapter(&server.uri());
    adapter.connect().await.expect("connect should succeed");
    assert!(adapter.is_connected());
}

#[tokio::test]
async fn connect_fails_on_invalid_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/bot123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11/getMe"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "ok": false,
            "error_code": 401,
            "description": "Unauthorized"
        })))
        .mount(&server)
        .await;

    let mut adapter = test_adapter(&server.uri());
    let result = adapter.connect().await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("authentication failed"), "got: {err}");
}

#[tokio::test]
async fn send_message_builds_correct_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/bot123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11/sendMessage"))
        .and(body_json(json!({
            "chat_id": 42,
            "text": "Build #100 passed",
            "parse_mode": "Markdown"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "result": { "message_id": 999, "date": 1711234567, "chat": {"id": 42} }
        })))
        .mount(&server)
        .await;

    let adapter = test_adapter(&server.uri());
    let msg = ChannelMessage {
        id: "msg-tg-001".into(),
        source_channel: "telegram".into(),
        content: "Build #100 passed".into(),
        reply_to: None,
        metadata: json!({}),
        timestamp: chrono::Utc::now(),
    };
    adapter.send(&msg).await.expect("send should succeed");
}

#[tokio::test]
async fn send_fails_without_chat_id() {
    let adapter = TelegramAdapter::new_with_base_url(
        "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11".into(),
        None,
        "http://localhost:1".into(),
    );
    let msg = ChannelMessage {
        id: "msg-tg-002".into(),
        source_channel: "telegram".into(),
        content: "Should fail".into(),
        reply_to: None,
        metadata: json!({}),
        timestamp: chrono::Utc::now(),
    };
    let result = adapter.send(&msg).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("chat_id"), "got: {err}");
}

#[tokio::test]
async fn get_updates_parses_messages() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/bot123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11/getUpdates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "result": [{
                "update_id": 100,
                "message": {
                    "message_id": 10,
                    "date": 1711234567,
                    "chat": {"id": 42, "type": "private"},
                    "from": {"id": 1, "first_name": "Roberto", "is_bot": false},
                    "text": "/status"
                }
            }]
        })))
        .mount(&server)
        .await;

    let mut adapter = test_adapter(&server.uri());
    let messages = adapter.poll_updates().await.expect("poll should succeed");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "/status");
    assert_eq!(messages[0].source_channel, "telegram");
}

#[tokio::test]
async fn command_routing_status() {
    let response = route_command("/status", &[]);
    assert!(response.contains("status"), "got: {response}");
}

#[tokio::test]
async fn command_routing_approve() {
    let response = route_command("/approve 42", &[]);
    assert!(response.contains("42"), "got: {response}");
    assert!(response.to_lowercase().contains("approv"), "got: {response}");
}

#[tokio::test]
async fn command_routing_reject() {
    let response = route_command("/reject 99", &[]);
    assert!(response.contains("99"), "got: {response}");
    assert!(response.to_lowercase().contains("reject"), "got: {response}");
}

#[tokio::test]
async fn command_routing_ask() {
    let response = route_command("/ask what is the plan status?", &[]);
    assert!(
        response.contains("what is the plan status?"),
        "got: {response}"
    );
}

#[tokio::test]
async fn command_routing_unknown_shows_help() {
    let response = route_command("hello bot", &[]);
    assert!(response.contains("/status"), "help should list commands: {response}");
}

#[tokio::test]
async fn health_reflects_connection_state() {
    let adapter = test_adapter("http://localhost:1");
    let health = adapter.health().await;
    assert!(!health.connected);
    assert_eq!(health.channel_name, "telegram");
    assert!(health.last_message_at.is_none());
    assert_eq!(health.error_count, 0);
}

#[test]
fn adapter_name_returns_telegram() {
    let adapter = test_adapter("http://localhost:1");
    assert_eq!(adapter.name(), "telegram");
}
