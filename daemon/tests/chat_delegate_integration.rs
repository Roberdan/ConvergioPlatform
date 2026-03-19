//! Integration tests for chat CRUD and delegation SSE with task status.
//! Marked #[ignore] because they bind a real TCP listener.

use claude_core::server::routes::build_router_with_db;
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio::net::TcpListener;

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

#[tokio::test]
#[ignore]
async fn chat_session_create_and_message_roundtrip() {
    let tmp = tempfile::TempDir::new().expect("tmpdir");
    let db_path = tmp.path().join("test.db");
    seed_db(&db_path);
    let addr = start_server(db_path).await;

    let client = reqwest::Client::new();
    let base = format!("http://{addr}");

    // Create chat session
    let res = client
        .post(format!("{base}/api/chat/session"))
        .json(&json!({"session_id": "test-session-42", "title": "Integration chat"}))
        .send()
        .await
        .expect("create session");
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.expect("json");
    assert_eq!(body["ok"], true);
    assert_eq!(body["session"]["id"], "test-session-42");
    assert_eq!(body["session"]["title"], "Integration chat");

    // Add user message
    let res = client
        .post(format!("{base}/api/chat/message"))
        .json(&json!({
            "session_id": "test-session-42",
            "role": "user",
            "content": "Explain the mesh sync protocol"
        }))
        .send()
        .await
        .expect("create message");
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.expect("json");
    assert_eq!(body["ok"], true);
    assert!(body["message_id"].as_i64().unwrap() > 0);

    // List sessions — should include our session
    let res = client
        .get(format!("{base}/api/chat/sessions"))
        .send()
        .await
        .expect("list sessions");
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.expect("json");
    let sessions = body["sessions"].as_array().expect("sessions array");
    assert!(
        sessions.iter().any(|s| s["id"] == "test-session-42"),
        "created session not found in list"
    );

    // Delete session
    let res = client
        .delete(format!("{base}/api/chat/session?sid=test-session-42"))
        .send()
        .await
        .expect("delete session");
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.expect("json");
    assert_eq!(body["deleted"], true);
}

#[tokio::test]
#[ignore]
async fn sse_delegate_with_task_status_check() {
    let tmp = tempfile::TempDir::new().expect("tmpdir");
    let db_path = tmp.path().join("test.db");
    seed_db(&db_path);
    let addr = start_server(db_path).await;

    let client = reqwest::Client::new();

    // Set task to in_progress first, then delegate
    client
        .post(format!("http://{addr}/api/plan-db/task/update"))
        .json(&json!({"task_id": 1, "status": "in_progress"}))
        .send()
        .await
        .expect("update task");

    // Delegate with nonexistent peer — verify error contains stage events
    let body = client
        .get(format!(
            "http://{addr}/api/plan/delegate?\
             plan_id=671&target=unreachable-worker&cli=claude&task_id=T1-01"
        ))
        .send()
        .await
        .expect("delegate")
        .text()
        .await
        .expect("body");

    assert!(body.contains("event: stage"), "missing stage event: {body}");
    // Peer resolution should fail gracefully
    assert!(
        body.contains("event: error"),
        "missing error on unreachable peer: {body}"
    );
}
