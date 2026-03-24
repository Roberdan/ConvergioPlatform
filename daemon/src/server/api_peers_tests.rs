//! Integration tests for /api/peers endpoints.
//! Covers: list, create, update, delete, ssh-check, discover.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn test_router() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!("claude-peers-test-{}-{n}.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(SEED_SCHEMA).expect("seed schema");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

const SEED_SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS peer_heartbeats (
    peer_name TEXT PRIMARY KEY,
    last_seen REAL,
    load_json TEXT,
    capabilities TEXT
);
INSERT INTO peer_heartbeats (peer_name, last_seen, capabilities)
  VALUES ('mac-worker-2', strftime('%s','now'), 'rust,python');
INSERT INTO peer_heartbeats (peer_name, last_seen, capabilities)
  VALUES ('remote-worker-1', 0, 'go');
";

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 65536).await.expect("bytes");
    serde_json::from_slice(&bytes).expect("json")
}

#[tokio::test]
async fn peer_list_returns_peers_array() {
    let app = test_router();
    let resp = app
        .oneshot(Request::get("/api/peers").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    let peers = json["peers"].as_array().expect("peers array");
    assert_eq!(peers.len(), 2);
}

#[tokio::test]
async fn peer_list_includes_is_online_and_role() {
    let app = test_router();
    let resp = app
        .oneshot(Request::get("/api/peers").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    let peers = json["peers"].as_array().unwrap();
    // mac-worker-2 should be coordinator + online (last_seen = now)
    let local = peers
        .iter()
        .find(|p| p["peer_name"] == "mac-worker-2")
        .expect("local peer");
    assert_eq!(local["role"], "coordinator");
    assert_eq!(local["is_local"], true);
    assert_eq!(local["is_online"], true);
    // remote-worker-1 should be worker + offline (last_seen = 0)
    let remote = peers
        .iter()
        .find(|p| p["peer_name"] == "remote-worker-1")
        .expect("remote peer");
    assert_eq!(remote["role"], "worker");
    assert_eq!(remote["is_online"], false);
}

#[tokio::test]
async fn peer_create_valid_returns_ok() {
    let app = test_router();
    let body = serde_json::json!({"peer_name": "new-peer"});
    let resp = app
        .oneshot(
            Request::post("/api/peers")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["peer"], "new-peer");
}

#[tokio::test]
async fn peer_create_empty_name_returns_error() {
    let app = test_router();
    let body = serde_json::json!({"peer_name": ""});
    let resp = app
        .oneshot(
            Request::post("/api/peers")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    assert!(json["error"].is_string(), "should return error message");
}

#[tokio::test]
async fn peer_create_missing_name_returns_error() {
    let app = test_router();
    let body = serde_json::json!({});
    let resp = app
        .oneshot(
            Request::post("/api/peers")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    assert!(json["error"].is_string());
}

#[tokio::test]
async fn peer_update_returns_ok() {
    let app = test_router();
    let body = serde_json::json!({"capabilities": "rust,go"});
    let resp = app
        .oneshot(
            Request::put("/api/peers/test-peer")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["peer"], "test-peer");
}

#[tokio::test]
async fn peer_delete_returns_ok() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::delete("/api/peers/old-peer")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["deleted"], true);
    assert_eq!(json["peer"], "old-peer");
}

#[tokio::test]
async fn peer_ssh_check_returns_unreachable() {
    let app = test_router();
    let body = serde_json::json!({"host": "example.com"});
    let resp = app
        .oneshot(
            Request::post("/api/peers/ssh-check")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], false);
    assert_eq!(json["latency_ms"], -1);
}

#[tokio::test]
async fn peer_discover_returns_empty() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::get("/api/peers/discover")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert!(json["discovered"].as_array().unwrap().is_empty());
}
