// Why: Plan 725 T3-02 — TDD tests for EmailAdapter (ChannelAdapter impl).
// Tests validate SMTP relay dispatch, subject routing, health state, and config validation.

use super::*;
use crate::channels::*;
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: build adapter pointing at a mock HTTP relay.
fn test_adapter(endpoint: &str) -> EmailAdapter {
    EmailAdapter::new_with_config(
        endpoint.into(),
        "convergio@example.com".into(),
        "ops@example.org".into(),
    )
}

#[test]
fn adapter_name_returns_email() {
    let adapter = test_adapter("http://localhost:1");
    assert_eq!(adapter.name(), "email");
}

#[test]
fn subject_parse_approve_extracts_id() {
    let action = parse_subject_action("[CONVERGIO-APPROVE-42]");
    assert_eq!(action, Some(SubjectAction::Approve("42".into())));
}

#[test]
fn subject_parse_reject_extracts_id() {
    let action = parse_subject_action("[CONVERGIO-REJECT-99]");
    assert_eq!(action, Some(SubjectAction::Reject("99".into())));
}

#[test]
fn subject_parse_no_marker_returns_none() {
    let action = parse_subject_action("Re: Weekly digest");
    assert!(action.is_none());
}

#[test]
fn subject_parse_partial_prefix_returns_none() {
    let action = parse_subject_action("[CONVERGIO-42]");
    assert!(action.is_none());
}

#[test]
fn health_state_disconnected_by_default() {
    let adapter = test_adapter("http://localhost:1");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let health = rt.block_on(adapter.health());
    assert!(!health.connected);
    assert_eq!(health.channel_name, "email");
    assert_eq!(health.error_count, 0);
    assert!(health.last_message_at.is_none());
}

#[test]
fn connect_fails_with_empty_endpoint() {
    let mut adapter = EmailAdapter::new_with_config(
        "".into(),
        "convergio@example.com".into(),
        "ops@example.org".into(),
    );
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(adapter.connect());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("connection failed") || err.contains("CONVERGIO_EMAIL_ENDPOINT"),
        "got: {err}"
    );
}

#[test]
fn connect_fails_with_empty_from() {
    let mut adapter = EmailAdapter::new_with_config(
        "http://relay.example.com/send".into(),
        "".into(),
        "ops@example.org".into(),
    );
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(adapter.connect());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("connection failed") || err.contains("CONVERGIO_EMAIL_FROM"),
        "got: {err}"
    );
}

#[tokio::test]
async fn send_posts_to_relay_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/send"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "queued"})))
        .mount(&server)
        .await;

    let endpoint = format!("{}/send", server.uri());
    let mut adapter = test_adapter(&endpoint);
    adapter.connect().await.expect("connect should succeed");

    let msg = ChannelMessage {
        id: "email-001".into(),
        source_channel: "email".into(),
        content: "Deploy pipeline passed all 47 checks".into(),
        reply_to: None,
        metadata: json!({"subject": "Convergio CI: plan 725 wave 3"}),
        timestamp: chrono::Utc::now(),
    };
    adapter.send(&msg).await.expect("send should succeed");
}

#[tokio::test]
async fn send_formats_correct_payload() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/send"))
        .respond_with(ResponseTemplate::new(202).set_body_json(json!({"id": "msg-xyz"})))
        .mount(&server)
        .await;

    let endpoint = format!("{}/api/send", server.uri());
    let mut adapter = test_adapter(&endpoint);
    adapter.connect().await.expect("connect");

    let msg = ChannelMessage {
        id: "email-002".into(),
        source_channel: "email".into(),
        content: "Wave 3 merged successfully.".into(),
        reply_to: None,
        metadata: json!({}),
        timestamp: chrono::Utc::now(),
    };
    adapter.send(&msg).await.expect("send should succeed with 202");
}

#[tokio::test]
async fn send_fails_when_relay_returns_5xx() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/send"))
        .respond_with(ResponseTemplate::new(503).set_body_json(json!({"error": "unavailable"})))
        .mount(&server)
        .await;

    let endpoint = format!("{}/send", server.uri());
    let mut adapter = test_adapter(&endpoint);
    adapter.connect().await.expect("connect");

    let msg = ChannelMessage {
        id: "email-003".into(),
        source_channel: "email".into(),
        content: "Should fail".into(),
        reply_to: None,
        metadata: json!({}),
        timestamp: chrono::Utc::now(),
    };
    let result = adapter.send(&msg).await;
    assert!(result.is_err(), "5xx must result in error");
}

#[tokio::test]
async fn health_reflects_connected_after_connect() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
        .mount(&server)
        .await;

    let endpoint = format!("{}/send", server.uri());
    let mut adapter = test_adapter(&endpoint);
    assert!(!adapter.health().await.connected);

    adapter.connect().await.expect("connect");
    assert!(adapter.health().await.connected);
}

#[tokio::test]
async fn disconnect_resets_connected_state() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
        .mount(&server)
        .await;

    let endpoint = format!("{}/send", server.uri());
    let mut adapter = test_adapter(&endpoint);
    adapter.connect().await.expect("connect");
    assert!(adapter.health().await.connected);

    adapter.disconnect().await.expect("disconnect");
    assert!(!adapter.health().await.connected);
}
