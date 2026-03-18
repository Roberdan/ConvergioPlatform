use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn test_router() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!("claude-ipc-test-{}-{n}.db", std::process::id()));
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

#[tokio::test]
async fn ipc_agents_returns_ok() {
    let app = test_router();
    let resp = app
        .oneshot(Request::get("/api/ipc/agents").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["ok"], true);
    assert!(body["agents"].is_array());
}

#[tokio::test]
async fn ipc_locks_returns_ok() {
    let app = test_router();
    let resp = app
        .oneshot(Request::get("/api/ipc/locks").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["ok"], true);
    assert!(body["locks"].is_array());
}

#[tokio::test]
async fn ipc_worktrees_returns_ok() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::get("/api/ipc/worktrees")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn ipc_status_returns_counts() {
    let app = test_router();
    let resp = app
        .oneshot(Request::get("/api/ipc/status").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["ok"], true);
    assert!(body["agents_active"].is_number());
    assert!(body["locks_active"].is_number());
}

#[tokio::test]
async fn ipc_send_creates_message() {
    let app = test_router();
    let payload = serde_json::json!({
        "channel": "test-channel",
        "content": "hello from test",
        "sender_name": "test-agent"
    });
    let resp = app
        .oneshot(
            Request::post("/api/ipc/send")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn ipc_channels_returns_ok() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::get("/api/ipc/channels")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["ok"], true);
    assert!(body["channels"].is_array());
}

#[tokio::test]
async fn ipc_context_returns_ok() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::get("/api/ipc/context")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn ipc_conflicts_returns_ok() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::get("/api/ipc/conflicts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["ok"], true);
}
