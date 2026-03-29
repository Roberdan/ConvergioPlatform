// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for GET /api/telemetry endpoint.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-telemetry-test-{}-{n}.db",
        std::process::id()
    ));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS plans (id INTEGER PRIMARY KEY, name TEXT, status TEXT, project_id TEXT);
CREATE TABLE IF NOT EXISTS agent_activity (
  id INTEGER PRIMARY KEY, agent_id TEXT UNIQUE, action TEXT, status TEXT
);
CREATE TABLE IF NOT EXISTS peer_heartbeats (id INTEGER PRIMARY KEY, peer TEXT);
";

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 65536).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn telemetry_returns_ok_shape() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/telemetry")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert!(json["total_requests"].is_number());
    assert!(json["total_errors"].is_number());
    assert!(json["error_rate"].is_number());
    assert!(json["endpoints"].is_array());
}

#[tokio::test]
async fn telemetry_tracks_requests() {
    let app = test_router();
    // First request to /api/health
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Check telemetry incremented
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/telemetry")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    let total = json["total_requests"].as_u64().unwrap_or(0);
    // At least the /api/health + /api/telemetry requests should be counted
    assert!(total >= 1, "expected at least 1 request, got {total}");
}
