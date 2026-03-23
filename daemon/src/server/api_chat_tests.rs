// Integration tests for chat API: session + models endpoints.
// Pattern: build_router_with_db with temp DB, send requests via oneshot.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn test_router() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp =
        std::env::temp_dir().join(format!("claude-chat-test-{}-{n}.db", std::process::id()));
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

async fn get(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
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

async fn delete(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .uri(uri)
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn chat_models_returns_list() {
    let r = test_router();
    let (s, j) = get(&r, "/api/chat/models").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    let models = j["models"].as_array().expect("models array");
    assert!(!models.is_empty(), "should return at least one model");
}

#[tokio::test]
async fn chat_sessions_list_empty() {
    let r = test_router();
    let (s, j) = get(&r, "/api/chat/sessions").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    let sessions = j["sessions"].as_array().expect("sessions array");
    assert!(sessions.is_empty(), "fresh DB should have no sessions");
}

#[tokio::test]
async fn chat_session_create_and_list() {
    let r = test_router();
    let (s, j) = post(
        &r,
        "/api/chat/session",
        serde_json::json!({"title": "Test Session"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["session"]["title"], "Test Session");
    assert_eq!(j["session"]["status"], "active");
    let sid = j["session"]["id"].as_str().expect("session id");
    assert!(!sid.is_empty());

    // List should now contain the session
    let (s, j) = get(&r, "/api/chat/sessions").await;
    assert_eq!(s, StatusCode::OK);
    let sessions = j["sessions"].as_array().expect("sessions");
    assert_eq!(sessions.len(), 1);
}

#[tokio::test]
async fn chat_session_create_with_custom_id() {
    let r = test_router();
    let (s, j) = post(
        &r,
        "/api/chat/session",
        serde_json::json!({"session_id": "custom-123", "title": "Custom"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["session"]["id"], "custom-123");
    assert_eq!(j["session"]["title"], "Custom");
}

#[tokio::test]
async fn chat_session_delete() {
    let r = test_router();
    post(
        &r,
        "/api/chat/session",
        serde_json::json!({"session_id": "del-sess", "title": "To Delete"}),
    )
    .await;

    let (s, j) = delete(&r, "/api/chat/session?sid=del-sess").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["deleted"], true);
    assert_eq!(j["session_id"], "del-sess");

    // Deleted session should not appear in list
    let (_, j) = get(&r, "/api/chat/sessions").await;
    let sessions = j["sessions"].as_array().expect("sessions");
    assert!(sessions.is_empty(), "deleted session should not appear");
}

#[tokio::test]
async fn chat_session_delete_missing_id() {
    let r = test_router();
    let (s, _) = delete(&r, "/api/chat/session").await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn chat_approve_returns_ok() {
    let r = test_router();
    let (s, j) = post(&r, "/api/chat/approve", serde_json::json!({})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
}

#[tokio::test]
async fn chat_execute_returns_queued() {
    let r = test_router();
    let (s, j) = post(&r, "/api/chat/execute", serde_json::json!({})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["queued"], true);
}
