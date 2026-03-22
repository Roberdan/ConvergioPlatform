// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Shared test helpers for cross-feature integration tests.

use super::state::ServerState;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::Value;
use tower::ServiceExt;

/// Core tables needed before ServerState migrations can run ALTER TABLE.
pub(super) const CORE_TABLES: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, path TEXT NOT NULL DEFAULT '',
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS plans (
  id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL,
  name TEXT NOT NULL, source_file TEXT, status TEXT NOT NULL DEFAULT 'todo',
  tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP, started_at DATETIME,
  completed_at DATETIME, description TEXT, human_summary TEXT,
  execution_host TEXT, parallel_mode TEXT, lines_added INTEGER,
  lines_removed INTEGER, cancelled_at DATETIME, cancelled_reason TEXT,
  updated_at DATETIME, worktree_path TEXT, constraints_json TEXT,
  is_master INTEGER DEFAULT 0, waves_total INTEGER DEFAULT 0,
  waves_merged INTEGER DEFAULT 0
);
CREATE TABLE IF NOT EXISTS waves (
  id INTEGER PRIMARY KEY AUTOINCREMENT, plan_id INTEGER NOT NULL,
  project_id TEXT, wave_id TEXT NOT NULL, name TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending', tasks_done INTEGER DEFAULT 0,
  tasks_total INTEGER DEFAULT 0, position INTEGER DEFAULT 0,
  started_at DATETIME, completed_at DATETIME, cancelled_at DATETIME,
  theme TEXT, depends_on TEXT, pr_number INTEGER, pr_url TEXT,
  cancelled_reason TEXT, merge_mode TEXT DEFAULT 'sync',
  estimated_hours INTEGER DEFAULT 8, worktree_path TEXT
);
CREATE TABLE IF NOT EXISTS tasks (
  id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL DEFAULT '',
  wave_id TEXT NOT NULL DEFAULT '', task_id TEXT NOT NULL DEFAULT '',
  title TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT 'pending',
  tokens INTEGER DEFAULT 0, wave_id_fk INTEGER, plan_id INTEGER,
  model TEXT DEFAULT 'haiku', output_data TEXT, started_at DATETIME,
  completed_at DATETIME, notes TEXT, output_type TEXT DEFAULT 'pr',
  validator_agent TEXT DEFAULT 'thor', effort_level INTEGER DEFAULT 1,
  validated_at DATETIME, validated_by TEXT, validation_report TEXT,
  priority TEXT, type TEXT, assignee TEXT, description TEXT,
  test_criteria TEXT, executor_host TEXT, executor_agent TEXT,
  duration_minutes REAL
);
CREATE TABLE IF NOT EXISTS knowledge_base (
  id INTEGER PRIMARY KEY, domain TEXT, title TEXT, content TEXT,
  created_at TEXT DEFAULT (datetime('now')), hit_count INTEGER DEFAULT 0
);
CREATE TABLE IF NOT EXISTS peer_heartbeats (
  peer_name TEXT PRIMARY KEY, last_seen INTEGER NOT NULL,
  load_json TEXT, capabilities TEXT
);
CREATE TABLE IF NOT EXISTS token_usage (
  id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT, plan_id INTEGER,
  model TEXT, input_tokens INTEGER DEFAULT 0, output_tokens INTEGER DEFAULT 0,
  cost_usd REAL DEFAULT 0, created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS mesh_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT, event_type TEXT NOT NULL,
  source_peer TEXT NOT NULL DEFAULT '', payload TEXT,
  status TEXT DEFAULT 'pending', created_at INTEGER DEFAULT (unixepoch())
);
CREATE TABLE IF NOT EXISTS notifications (
  id INTEGER PRIMARY KEY AUTOINCREMENT, type TEXT NOT NULL DEFAULT '',
  title TEXT NOT NULL DEFAULT '', message TEXT NOT NULL DEFAULT '',
  is_read INTEGER DEFAULT 0, created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
";

/// Create a ServerState with core tables and a seeded project.
pub(super) fn setup_state(project_id: &str, project_name: &str) -> (ServerState, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch(CORE_TABLES).unwrap();
    conn.execute(
        "INSERT INTO projects(id, name, path) VALUES(?1, ?2, ?3)",
        rusqlite::params![project_id, project_name, tmp.path().to_string_lossy()],
    )
    .unwrap();
    drop(conn);
    let state = ServerState::new(db_path, None);
    (state, tmp)
}

pub(super) async fn post_json(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

pub(super) async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}
