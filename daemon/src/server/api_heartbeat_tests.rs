// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for heartbeat + watchdog API endpoints.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!("claude-hb-test-{}-{n}.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS peer_heartbeats (
  peer_name TEXT PRIMARY KEY, last_seen REAL,
  load_json TEXT, capabilities TEXT
);
CREATE TABLE IF NOT EXISTS host_heartbeats (
  hostname TEXT PRIMARY KEY, last_seen TEXT,
  status TEXT, metadata TEXT
);
CREATE TABLE IF NOT EXISTS tasks (
  id INTEGER PRIMARY KEY, task_id TEXT, title TEXT,
  status TEXT, started_at TEXT, plan_id INTEGER
);
CREATE TABLE IF NOT EXISTS agent_activity (
  id INTEGER PRIMARY KEY, agent_id TEXT NOT NULL,
  agent_type TEXT NOT NULL DEFAULT 'legacy',
  status TEXT NOT NULL DEFAULT 'completed',
  started_at TEXT DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS notification_queue (
  id INTEGER PRIMARY KEY, status TEXT DEFAULT 'pending'
);
CREATE TABLE IF NOT EXISTS plans (
  id INTEGER PRIMARY KEY, name TEXT, status TEXT
);
";

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 65536).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// --- POST /api/heartbeat ----------------------------------------------------

#[tokio::test]
async fn heartbeat_post_returns_ok_and_peer_name() {
    let app = test_router();
    let body = serde_json::json!({"peer_name": "node-alpha"});
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/heartbeat")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["peer_name"], "node-alpha");
    assert!(json["timestamp"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn heartbeat_post_without_peer_name_uses_hostname() {
    let app = test_router();
    let body = serde_json::json!({});
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/heartbeat")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    // peer_name should be the machine hostname, not empty
    assert!(!json["peer_name"].as_str().unwrap().is_empty());
}

// --- GET /api/heartbeat/status ----------------------------------------------

#[tokio::test]
async fn heartbeat_status_empty_returns_ok() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/heartbeat/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert!(json["peers"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn heartbeat_status_shows_posted_peer() {
    let app = test_router();

    // Post a heartbeat first
    let body = serde_json::json!({"peer_name": "worker-1"});
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/heartbeat")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Check status
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/heartbeat/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    let peers = json["peers"].as_array().unwrap();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0]["peer_name"], "worker-1");
    assert_eq!(peers[0]["status"], "healthy");
}

// --- GET /api/watchdog/status -----------------------------------------------

#[tokio::test]
async fn watchdog_status_healthy_when_empty() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/watchdog/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["healthy"], true);
    assert_eq!(json["stale_tasks"], 0);
    assert_eq!(json["orphan_agents"], 0);
    assert!(json["uptime_secs"].as_u64().is_some());
    assert!(json["system"].is_object());
}

// --- GET /api/watchdog/diagnostics ------------------------------------------

#[tokio::test]
async fn watchdog_diagnostics_response_shape() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/watchdog/diagnostics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert!(json["stale_tasks"].is_array());
    assert!(json["orphan_agents"].is_array());
    assert!(json["pending_notifications"].is_number());
    assert!(json["active_plans"].is_number());
    assert!(json["version"].is_string());
}
