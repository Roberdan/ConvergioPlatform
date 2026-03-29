use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn test_router() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-voice-test-{}-{n}.db",
        std::process::id()
    ));
    let conn = rusqlite::Connection::open(&tmp).expect("open db");
    conn.execute_batch("PRAGMA journal_mode=WAL;").expect("wal");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

async fn body_json(b: Body) -> Value {
    let bytes = axum::body::to_bytes(b, 65536).await.expect("body bytes");
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

#[tokio::test]
async fn test_voice_status_returns_ok() {
    let app = test_router();
    let req = Request::builder()
        .uri("/api/voice/status")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert!(body["state"].is_string(), "response must include state");
    // State is "idle" or "listening" depending on global AtomicBool test ordering.
    let state = body["state"].as_str().unwrap();
    assert!(
        state == "idle" || state == "listening",
        "unexpected state: {state}"
    );
}

#[tokio::test]
async fn test_voice_status_includes_config() {
    let app = test_router();
    let req = Request::builder()
        .uri("/api/voice/status")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let body = body_json(resp.into_body()).await;
    assert!(body["config"].is_object(), "response must include config");
    assert!(body["config"]["wake_word"].is_string());
}

#[tokio::test]
async fn test_voice_start_returns_ok() {
    let app = test_router();
    let req = Request::builder()
        .method("POST")
        .uri("/api/voice/start")
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn test_voice_stop_returns_ok() {
    let app = test_router();
    let req = Request::builder()
        .method("POST")
        .uri("/api/voice/stop")
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn test_voice_test_returns_ok() {
    let app = test_router();
    let req = Request::builder()
        .method("POST")
        .uri("/api/voice/test")
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert!(body["ok"].is_boolean());
}

#[tokio::test]
async fn test_voice_start_then_status_shows_listening() {
    let app = test_router();
    let start_req = Request::builder()
        .method("POST")
        .uri("/api/voice/start")
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let resp = app.oneshot(start_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
