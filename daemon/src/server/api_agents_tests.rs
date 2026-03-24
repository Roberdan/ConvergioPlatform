// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for /api/agents and /api/sessions endpoints.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

pub(super) fn test_router() -> (axum::Router, std::path::PathBuf) {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp =
        std::env::temp_dir().join(format!("claude-agents-test-{}-{n}.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    drop(conn);
    super::middleware::set_dev_mode(true);
    let router =
        super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp.clone(), None);
    (router, tmp)
}

pub(super) const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS plans (
  id INTEGER PRIMARY KEY, name TEXT, status TEXT,
  execution_host TEXT, tasks_done INTEGER DEFAULT 0,
  tasks_total INTEGER DEFAULT 0
);
CREATE TABLE IF NOT EXISTS tasks (
  id INTEGER PRIMARY KEY, task_id TEXT, title TEXT,
  status TEXT, assignee TEXT, priority TEXT,
  type TEXT, tokens INTEGER, started_at TEXT,
  executor_session_id TEXT, executor_host TEXT,
  model TEXT, wave_id TEXT, output_data TEXT,
  plan_id INTEGER, wave_id_fk INTEGER
);
CREATE TABLE IF NOT EXISTS waves (
  id INTEGER PRIMARY KEY, plan_id INTEGER, name TEXT
);
CREATE TABLE IF NOT EXISTS plan_commits (
  id INTEGER PRIMARY KEY, plan_id INTEGER,
  commit_sha TEXT, commit_message TEXT,
  lines_added INTEGER DEFAULT 0, lines_removed INTEGER DEFAULT 0,
  files_changed INTEGER DEFAULT 0, authored_at TEXT
);
";

pub(super) fn seed_db(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).expect("open for seed");
    conn.execute_batch(
        "INSERT INTO plans(id,name,status,tasks_done,tasks_total) \
         VALUES(1,'Deploy Alpha','doing',2,5);",
    )
    .expect("seed plans");
    conn.execute(
        "INSERT OR REPLACE INTO agent_activity \
         (agent_id, agent_type, model, description, status, started_at, \
          tokens_in, tokens_out, tokens_total, cost_usd, host, region) \
         VALUES(?1,?2,?3,?4,'running',datetime('now'),\
          100,200,300,0.05,'local','prefrontal')",
        rusqlite::params!["session-cli-001", "copilot-cli", "opus", "Primary session"],
    )
    .unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO agent_activity \
         (agent_id, agent_type, model, description, status, started_at, \
          tokens_total, cost_usd, parent_session, plan_id) \
         VALUES(?1,?2,?3,?4,'running',datetime('now'),150,0.03,?5,1)",
        rusqlite::params![
            "worker-A",
            "task",
            "sonnet",
            "Sub-agent worker",
            "session-cli-001"
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO agent_activity \
         (agent_id, agent_type, model, description, status, started_at, \
          completed_at, tokens_total, cost_usd, duration_s) \
         VALUES(?1,?2,?3,?4,'completed',datetime('now','-30 minutes'),\
          datetime('now','-25 minutes'),500,0.10,300.0)",
        rusqlite::params!["worker-done", "task", "haiku", "Completed worker"],
    )
    .unwrap();
}

pub(super) async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 131072).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// --- GET /api/agents -------------------------------------------------------

#[tokio::test]
async fn agents_returns_response_shape() {
    let (app, db) = test_router();
    seed_db(&db);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert!(json["running"].is_array());
    assert!(json["recent"].is_array());
    assert!(json["stats"].is_object());
    assert!(json["stats"]["active_count"].is_number());
    assert!(json["stats"]["by_model"].is_array());
}

#[tokio::test]
async fn agents_running_contains_seeded_agents() {
    let (app, db) = test_router();
    seed_db(&db);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    let running = json["running"].as_array().unwrap();
    assert_eq!(running.len(), 2);
    let ids: Vec<&str> = running
        .iter()
        .filter_map(|a| a["agent_id"].as_str())
        .collect();
    assert!(ids.contains(&"session-cli-001"));
    assert!(ids.contains(&"worker-A"));
}

#[tokio::test]
async fn agents_recent_contains_completed() {
    let (app, db) = test_router();
    seed_db(&db);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    let recent = json["recent"].as_array().unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0]["agent_id"], "worker-done");
    assert_eq!(recent[0]["status"], "completed");
}

#[tokio::test]
async fn agents_empty_db_returns_empty_arrays() {
    let (app, _db) = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["running"].as_array().unwrap().len(), 0);
    assert_eq!(json["stats"]["active_count"], 0);
}

// --- GET /api/sessions -----------------------------------------------------

#[tokio::test]
async fn sessions_returns_only_session_agents() {
    let (app, db) = test_router();
    seed_db(&db);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    let sessions = json.as_array().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["agent_id"], "session-cli-001");
    assert_eq!(sessions[0]["type"], "copilot-cli");
}
