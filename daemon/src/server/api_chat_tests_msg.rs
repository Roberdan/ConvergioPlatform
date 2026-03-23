// Integration tests for chat API: message + requirement endpoints.
// Pattern: build_router_with_db with temp DB, send requests via oneshot.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn test_router() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(200);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp =
        std::env::temp_dir().join(format!("claude-chatmsg-test-{}-{n}.db", std::process::id()));
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

async fn post(router: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .uri(uri)
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn put(router: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .uri(uri)
        .method("PUT")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn chat_message_create_success() {
    let r = test_router();
    // Create session first
    let (_, j) = post(
        &r,
        "/api/chat/session",
        serde_json::json!({"session_id": "msg-sess"}),
    )
    .await;
    assert_eq!(j["ok"], true);

    // Create message
    let (s, j) = post(
        &r,
        "/api/chat/message",
        serde_json::json!({
            "session_id": "msg-sess",
            "content": "Hello world",
            "role": "user"
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(j["message_id"].is_number(), "should return message_id");
}

#[tokio::test]
async fn chat_message_missing_session_id() {
    let r = test_router();
    let (s, _) = post(
        &r,
        "/api/chat/message",
        serde_json::json!({"content": "hello"}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn chat_message_empty_content() {
    let r = test_router();
    let (s, _) = post(
        &r,
        "/api/chat/message",
        serde_json::json!({"session_id": "s1", "content": "  "}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn chat_message_default_role() {
    let r = test_router();
    post(
        &r,
        "/api/chat/session",
        serde_json::json!({"session_id": "role-sess"}),
    )
    .await;

    // Omit role -- should default to "user"
    let (s, j) = post(
        &r,
        "/api/chat/message",
        serde_json::json!({"session_id": "role-sess", "content": "test"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
}

#[tokio::test]
async fn chat_requirement_upsert() {
    let r = test_router();
    let (s, j) = put(
        &r,
        "/api/chat/requirement",
        serde_json::json!({
            "session_id": "req-sess",
            "requirement_key": "REQ-001",
            "requirement_text": "Must handle edge cases"
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["requirement_key"], "REQ-001");
    assert_eq!(j["action"], "upsert");
}

#[tokio::test]
async fn chat_requirement_missing_fields() {
    let r = test_router();
    let (s, _) = put(
        &r,
        "/api/chat/requirement",
        serde_json::json!({"session_id": "s1"}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}
