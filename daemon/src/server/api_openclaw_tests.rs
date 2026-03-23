// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for OpenClaw bridge API endpoints.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn test_router() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-openclaw-test-{}-{n}.db",
        std::process::id()
    ));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(CORE_SCHEMA).expect("core schema");
    conn.execute_batch(SEED_DATA).expect("seed data");
    drop(conn);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

const CORE_SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS agent_catalog (
  name TEXT PRIMARY KEY,
  category TEXT,
  description TEXT,
  model TEXT,
  tools TEXT,
  skills TEXT,
  source_repo TEXT,
  constitution_version TEXT,
  version TEXT,
  created_at DATETIME DEFAULT (datetime('now')),
  updated_at DATETIME DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS session_state (
  key TEXT PRIMARY KEY,
  value TEXT
);
CREATE TABLE IF NOT EXISTS ipc_agent_skills (
  id INTEGER PRIMARY KEY,
  agent TEXT,
  host TEXT,
  skill TEXT,
  confidence REAL DEFAULT 0.5,
  last_used TEXT,
  UNIQUE(agent, host, skill)
);
";

const SEED_DATA: &str = "
INSERT INTO agent_catalog(name, category, description, model, tools)
  VALUES('test-agent', 'technical', 'Rust code reviewer', 'claude-sonnet-4-6', 'view,edit');
INSERT INTO agent_catalog(name, category, description, model, tools)
  VALUES('ali-orchestrator', 'leadership', 'Chief of staff orchestrator', 'claude-opus-4-6', 'bash,view');
";

async fn get(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
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
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn test_list_agents() {
    let r = test_router();
    let (s, j) = get(&r, "/api/openclaw/agents").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    let agents = j["agents"].as_array().expect("agents array");
    assert_eq!(agents.len(), 2, "seed data has 2 agents");
}

#[tokio::test]
async fn test_list_agents_fields() {
    let r = test_router();
    let (s, j) = get(&r, "/api/openclaw/agents").await;
    assert_eq!(s, StatusCode::OK);
    let agents = j["agents"].as_array().expect("agents array");
    for agent in agents {
        assert!(agent["name"].is_string(), "agent must have name");
        assert!(
            agent["description"].is_string(),
            "agent must have description"
        );
        assert!(agent["model"].is_string(), "agent must have model");
    }
}

#[tokio::test]
async fn test_invoke_with_agent() {
    let r = test_router();
    let (s, j) = post(
        &r,
        "/api/openclaw/invoke",
        serde_json::json!({"agent_id": "test-agent", "message": "review this code"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(j["request_id"].is_string(), "response must have request_id");
    assert_eq!(j["agent"], "test-agent");
}

#[tokio::test]
async fn test_invoke_default_agent() {
    let r = test_router();
    let (s, j) = post(
        &r,
        "/api/openclaw/invoke",
        serde_json::json!({"message": "help me"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["agent"], "ali-orchestrator");
}

#[tokio::test]
async fn test_invoke_empty_message() {
    let r = test_router();
    let (s, j) = post(
        &r,
        "/api/openclaw/invoke",
        serde_json::json!({"message": ""}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert!(
        j.to_string().contains("empty")
            || j.to_string().contains("bad_request")
            || s == StatusCode::BAD_REQUEST,
        "empty message should be rejected"
    );
}
