// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for agent control API (interrupt + reschedule).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS ipc_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sender TEXT, recipient TEXT, content TEXT,
    msg_type TEXT, created_at TEXT
);
CREATE TABLE IF NOT EXISTS agent_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL, agent_name TEXT,
    agent_type TEXT NOT NULL DEFAULT 'legacy',
    model TEXT, description TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    tokens_in INTEGER DEFAULT 0, tokens_out INTEGER DEFAULT 0,
    tokens_total INTEGER DEFAULT 0, cost_usd REAL DEFAULT 0,
    started_at TEXT DEFAULT (datetime('now')),
    completed_at TEXT, duration_s REAL, host TEXT, region TEXT,
    metadata TEXT, parent_session TEXT, notes TEXT,
    task_db_id INTEGER, plan_id INTEGER
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_agent_activity_agent_id
    ON agent_activity(agent_id);
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id_fk INTEGER,
    status TEXT DEFAULT 'pending', notes TEXT, project_id TEXT
);
CREATE TABLE IF NOT EXISTS delegation_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plan_id INTEGER, executor_agent TEXT,
    task_id INTEGER, cost_estimate REAL, timestamp TEXT
);
CREATE TABLE IF NOT EXISTS plans (
    id INTEGER PRIMARY KEY, name TEXT, status TEXT, project_id TEXT
);
";

const SEED: &str = "
INSERT INTO agent_activity (agent_id, agent_name, agent_type, model, status)
    VALUES ('sess-cli-001', 'task-executor-m5max', 'executor', 'claude-opus-4-6', 'running');
INSERT INTO plans (id, name, status, project_id)
    VALUES (1, 'Hardening Plan', 'doing', 'convergio');
INSERT INTO tasks (id, plan_id, status, project_id)
    VALUES (100, 1, 'in_progress', 'convergio'),
           (101, 1, 'blocked', 'convergio'),
           (102, 1, 'done', 'convergio');
";

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-agentctl-test-{}-{n}.db",
        std::process::id()
    ));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    conn.execute_batch(SEED).expect("seed");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

async fn post_json(router: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
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

// --- POST /api/agent/interrupt ---

#[tokio::test]
async fn interrupt_running_agent() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/agent/interrupt",
        serde_json::json!({
            "agent_name": "task-executor-m5max",
            "reason": "stall detected by kernel monitor"
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["interrupted"], true);
    assert_eq!(j["agent"], "task-executor-m5max");
}

#[tokio::test]
async fn interrupt_nonexistent_agent_returns_false() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/agent/interrupt",
        serde_json::json!({
            "agent_name": "phantom-agent",
            "reason": "does not exist"
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["interrupted"], false, "no running agent to interrupt");
}

#[tokio::test]
async fn interrupt_missing_fields_returns_422() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/agent/interrupt",
        serde_json::json!({"agent_name": "x"}),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
}

// --- POST /api/task/reschedule ---

#[tokio::test]
async fn reschedule_in_progress_task() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/task/reschedule",
        serde_json::json!({
            "task_id": 100,
            "from_node": "m5max-worker",
            "to_node": "m1pro-kernel",
            "reason": "node overloaded"
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["rescheduled"], true);
    assert_eq!(j["to_node"], "m1pro-kernel");
}

#[tokio::test]
async fn reschedule_blocked_task() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/task/reschedule",
        serde_json::json!({
            "task_id": 101,
            "from_node": "m5max-worker",
            "to_node": "linux-executor",
            "reason": "unblock by moving"
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["rescheduled"], true);
}

#[tokio::test]
async fn reschedule_done_task_returns_false() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/task/reschedule",
        serde_json::json!({
            "task_id": 102,
            "from_node": "m5max-worker",
            "to_node": "m1pro-kernel",
            "reason": "already done"
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["rescheduled"], false, "done tasks cannot be rescheduled");
}

#[tokio::test]
async fn reschedule_nonexistent_task_returns_false() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/task/reschedule",
        serde_json::json!({
            "task_id": 9999,
            "from_node": "x",
            "to_node": "y",
            "reason": "phantom"
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["rescheduled"], false);
}

#[tokio::test]
async fn reschedule_missing_fields_returns_422() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/task/reschedule",
        serde_json::json!({"task_id": 100}),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
}
