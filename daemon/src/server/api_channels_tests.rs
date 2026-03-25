// Integration tests for channel API endpoints.
// Uses in-memory SQLite so tests are self-contained and fast.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn test_router() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-channels-test-{}-{n}.db",
        std::process::id()
    ));
    let conn = rusqlite::Connection::open(&tmp).expect("open db");
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS notification_queue (
             id INTEGER PRIMARY KEY, severity TEXT DEFAULT 'info',
             title TEXT NOT NULL DEFAULT '', message TEXT,
             plan_id INTEGER, link TEXT, status TEXT DEFAULT 'pending',
             created_at TEXT DEFAULT (datetime('now')),
             delivered_at TEXT
         );",
    )
    .expect("create schema");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

async fn body_json(b: Body) -> Value {
    let bytes = axum::body::to_bytes(b, 65536).await.expect("body bytes");
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

#[tokio::test]
async fn test_list_channels_returns_ok_with_channels_array() {
    let app = test_router();
    let req = Request::builder()
        .uri("/api/channels")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["ok"], true);
    let channels = body["channels"].as_array().expect("channels array");
    assert!(!channels.is_empty(), "should have at least ntfy channel");
    // ntfy is always present
    let ntfy = channels.iter().find(|c| c["name"] == "ntfy");
    assert!(ntfy.is_some(), "ntfy channel should be present");
    let ntfy = ntfy.unwrap();
    assert!(ntfy.get("connected").is_some());
    assert!(ntfy.get("error_count").is_some());
}

#[tokio::test]
async fn test_send_message_to_valid_channel() {
    let app = test_router();
    let payload = serde_json::json!({
        "message": "Integration test message from Convergio",
        "severity": "info"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/channels/ntfy/send")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["channel"], "ntfy");
    assert!(body.get("delivered").is_some());
}

#[tokio::test]
async fn test_send_message_to_unknown_channel_returns_404() {
    let app = test_router();
    let payload = serde_json::json!({
        "message": "Should fail",
        "severity": "info"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/channels/nonexistent/send")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_channel_health_returns_health_struct() {
    let app = test_router();
    let req = Request::builder()
        .uri("/api/channels/ntfy/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["channel_name"], "ntfy");
    assert!(body.get("connected").is_some());
    assert!(body.get("error_count").is_some());
}

#[tokio::test]
async fn test_channel_health_unknown_returns_404() {
    let app = test_router();
    let req = Request::builder()
        .uri("/api/channels/nonexistent/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_send_missing_message_returns_400() {
    let app = test_router();
    let payload = serde_json::json!({
        "severity": "info"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/channels/ntfy/send")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
