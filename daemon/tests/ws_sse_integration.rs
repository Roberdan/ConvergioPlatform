//! Integration tests for WS brain push and SSE preflight/delegate streams.
//! Marked #[ignore] because they bind a real TCP listener.

use claude_core::server::routes::build_router_with_db;
use futures_util::StreamExt;
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

fn seed_db(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).expect("open db");
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT);
         CREATE TABLE plans (id INTEGER PRIMARY KEY, project_id TEXT DEFAULT '',
             name TEXT DEFAULT '', status TEXT DEFAULT 'todo', source_file TEXT,
             description TEXT, human_summary TEXT, tasks_total INTEGER DEFAULT 0,
             tasks_done INTEGER DEFAULT 0, execution_host TEXT, worktree_path TEXT,
             parallel_mode TEXT, created_at TEXT, started_at TEXT, completed_at TEXT,
             updated_at TEXT, cancelled_at TEXT, cancelled_reason TEXT,
             constraints_json TEXT, is_master INTEGER DEFAULT 0);
         CREATE TABLE waves (id INTEGER PRIMARY KEY, plan_id INTEGER,
             project_id TEXT DEFAULT '', wave_id TEXT, name TEXT,
             status TEXT DEFAULT 'pending', tasks_done INTEGER DEFAULT 0,
             tasks_total INTEGER DEFAULT 0, position INTEGER DEFAULT 0,
             depends_on TEXT, estimated_hours INTEGER DEFAULT 8, worktree_path TEXT,
             started_at TEXT, completed_at TEXT, cancelled_at TEXT,
             cancelled_reason TEXT, merge_mode TEXT DEFAULT 'sync', theme TEXT);
         CREATE TABLE tasks (id INTEGER PRIMARY KEY, project_id TEXT DEFAULT '',
             plan_id INTEGER, wave_id_fk INTEGER, wave_id TEXT, task_id TEXT,
             title TEXT, status TEXT DEFAULT 'pending', priority TEXT, type TEXT,
             assignee TEXT, description TEXT, test_criteria TEXT, model TEXT,
             notes TEXT, tokens INTEGER DEFAULT 0, output_data TEXT,
             executor_host TEXT, started_at TEXT, completed_at TEXT,
             validated_at TEXT, validated_by TEXT, validation_report TEXT);
         CREATE TABLE knowledge_base (id INTEGER PRIMARY KEY, domain TEXT,
             title TEXT, content TEXT, created_at TEXT, hit_count INTEGER DEFAULT 0);
         CREATE TABLE agent_activity (id INTEGER PRIMARY KEY AUTOINCREMENT,
             agent_id TEXT NOT NULL, agent_type TEXT NOT NULL DEFAULT 'legacy',
             model TEXT, description TEXT, status TEXT NOT NULL DEFAULT 'running',
             tokens_in INTEGER DEFAULT 0, tokens_out INTEGER DEFAULT 0,
             tokens_total INTEGER DEFAULT 0, cost_usd REAL DEFAULT 0,
             started_at TEXT DEFAULT (datetime('now')), completed_at TEXT,
             duration_s REAL, host TEXT, region TEXT DEFAULT 'prefrontal',
             metadata TEXT, task_db_id INTEGER, plan_id INTEGER, parent_session TEXT);
         CREATE UNIQUE INDEX IF NOT EXISTS uq_aa ON agent_activity(agent_id);
         CREATE TABLE ipc_agents (name TEXT PRIMARY KEY, host TEXT,
             agent_type TEXT, pid INTEGER, metadata TEXT,
             registered_at TEXT, last_seen TEXT);
         CREATE TABLE peer_heartbeats (peer_name TEXT, last_seen INTEGER,
             load_json TEXT);
         INSERT INTO projects (id, name) VALUES ('convergio', 'Convergio');
         INSERT INTO plans (id, project_id, name, status)
             VALUES (671, 'convergio', 'Plan 671', 'doing');
         INSERT INTO tasks (id, plan_id, wave_id, task_id, title, status)
             VALUES (1, 671, 'W1', 'T1-01', 'First task', 'pending');",
    )
    .expect("seed");
}

async fn start_server(db_path: std::path::PathBuf) -> SocketAddr {
    let static_dir = db_path.parent().unwrap().join("static");
    std::fs::create_dir_all(&static_dir).expect("mkdir static");
    let app = build_router_with_db(static_dir, db_path, None);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
    addr
}

/// Read next WS text message, parse as JSON.
async fn next_ws_json(
    ws: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> Value {
    let msg = tokio::time::timeout(TIMEOUT, ws.next())
        .await
        .expect("ws timeout")
        .expect("stream ended")
        .expect("ws error");
    serde_json::from_str(&msg.into_text().expect("text msg")).expect("json parse")
}

#[tokio::test]
#[ignore]
async fn ws_brain_receives_init_and_task_update() {
    let tmp = tempfile::TempDir::new().expect("tmpdir");
    let db_path = tmp.path().join("test.db");
    seed_db(&db_path);
    let addr = start_server(db_path).await;

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws/brain"))
        .await
        .expect("ws connect");

    // First message: heartbeat_snapshot init
    let init = next_ws_json(&mut ws).await;
    assert_eq!(init["kind"], "heartbeat_snapshot", "init event kind");

    // Update task status via HTTP — triggers broadcast_brain_task_update
    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://{addr}/api/plan-db/task/update"))
        .json(&json!({"task_id": 1, "status": "in_progress"}))
        .send()
        .await
        .expect("task update");
    assert_eq!(res.status(), StatusCode::OK);

    // WS should receive brain_event with task_update
    let update = next_ws_json(&mut ws).await;
    assert_eq!(update["kind"], "brain_event");
    assert_eq!(update["event_type"], "task_update");
    assert_eq!(update["payload"]["task_id"], 1);
    assert_eq!(update["payload"]["status"], "in_progress");
    assert_eq!(update["payload"]["plan_id"], 671);

    ws.close(None).await.ok();
}

#[tokio::test]
#[ignore]
async fn sse_plan_preflight_emits_real_check_events() {
    let tmp = tempfile::TempDir::new().expect("tmpdir");
    let db_path = tmp.path().join("test.db");
    seed_db(&db_path);
    let addr = start_server(db_path).await;

    let body = reqwest::Client::new()
        .get(format!(
            "http://{addr}/api/plan/preflight?plan_id=671&target=all"
        ))
        .send()
        .await
        .expect("preflight")
        .text()
        .await
        .expect("body");

    assert!(body.contains("event: start"), "missing start event");
    assert!(body.contains("event: done"), "missing done event");
    // Real plan_status check (not simulated)
    assert!(body.contains("plan_status"), "missing plan_status check");

    // Verify done event has correct structure
    let done_data: Value = body
        .lines()
        .skip_while(|l| !l.starts_with("event: done"))
        .nth(1)
        .map(|l| serde_json::from_str(l.trim_start_matches("data: ")).expect("parse done"))
        .expect("done data line");
    assert!(done_data.get("ok").is_some(), "done missing 'ok'");
    assert!(done_data.get("passed").is_some(), "done missing 'passed'");
    assert!(done_data.get("failed").is_some(), "done missing 'failed'");
    assert!(done_data.get("total").is_some(), "done missing 'total'");
}

#[tokio::test]
#[ignore]
async fn sse_plan_delegate_emits_stage_events() {
    let tmp = tempfile::TempDir::new().expect("tmpdir");
    let db_path = tmp.path().join("test.db");
    seed_db(&db_path);
    let addr = start_server(db_path).await;

    // Delegate to nonexistent peer — should emit stage then error
    let body = reqwest::Client::new()
        .get(format!(
            "http://{addr}/api/plan/delegate?\
             plan_id=671&target=nonexistent-peer&cli=claude&task_id=T1-01"
        ))
        .send()
        .await
        .expect("delegate")
        .text()
        .await
        .expect("body");

    assert!(body.contains("event: stage"), "missing stage event: {body}");
    assert!(
        body.contains("event: error"),
        "missing error event on unresolvable peer: {body}"
    );
}

#[tokio::test]
#[ignore]
async fn ws_brain_receives_sequential_task_updates() {
    let tmp = tempfile::TempDir::new().expect("tmpdir");
    let db_path = tmp.path().join("test.db");
    seed_db(&db_path);
    let addr = start_server(db_path).await;

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws/brain"))
        .await
        .expect("ws connect");
    let _ = next_ws_json(&mut ws).await; // consume init

    let client = reqwest::Client::new();
    let url = format!("http://{addr}/api/plan-db/task/update");

    // pending -> in_progress
    client
        .post(&url)
        .json(&json!({"task_id": 1, "status": "in_progress"}))
        .send()
        .await
        .expect("update 1");
    let ev1 = next_ws_json(&mut ws).await;
    assert_eq!(ev1["event_type"], "task_update");
    assert_eq!(ev1["payload"]["status"], "in_progress");

    // in_progress -> done
    client
        .post(&url)
        .json(&json!({"task_id": 1, "status": "done"}))
        .send()
        .await
        .expect("update 2");
    let ev2 = next_ws_json(&mut ws).await;
    assert_eq!(ev2["event_type"], "task_update");
    assert_eq!(ev2["payload"]["status"], "done");
    assert_eq!(ev2["payload"]["plan_id"], 671);

    ws.close(None).await.ok();
}
