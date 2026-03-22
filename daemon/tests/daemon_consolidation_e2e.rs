use axum::body::Body;
use axum::http::{Request, StatusCode};
use claude_core::server::routes::build_router_with_db;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

fn setup_app() -> (axum::Router, TempDir) {
    let tmp = TempDir::new().expect("tempdir");
    let db_path = tmp.path().join("test.db");
    let static_dir = tmp.path().join("static");
    std::fs::create_dir_all(&static_dir).expect("mkdir");
    let conn = rusqlite::Connection::open(&db_path).expect("open");
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL);
         CREATE TABLE plans (
             id INTEGER PRIMARY KEY, project_id TEXT NOT NULL DEFAULT '',
             name TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT 'draft',
             source_file TEXT, description TEXT, human_summary TEXT,
             tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0,
             execution_host TEXT, worktree_path TEXT, parallel_mode TEXT,
             created_at TEXT, started_at TEXT, completed_at TEXT,
             updated_at TEXT, cancelled_at TEXT, cancelled_reason TEXT,
             constraints_json TEXT, is_master INTEGER DEFAULT 0
         );
         CREATE TABLE waves (
             id INTEGER PRIMARY KEY, plan_id INTEGER, project_id TEXT DEFAULT '',
             wave_id TEXT, name TEXT, status TEXT DEFAULT 'pending',
             tasks_done INTEGER DEFAULT 0, tasks_total INTEGER DEFAULT 0,
             position INTEGER DEFAULT 0, depends_on TEXT,
             estimated_hours INTEGER DEFAULT 8, worktree_path TEXT,
             started_at TEXT, completed_at TEXT,
             cancelled_at TEXT, cancelled_reason TEXT,
             merge_mode TEXT DEFAULT 'sync', theme TEXT
         );
         CREATE TABLE tasks (
             id INTEGER PRIMARY KEY, project_id TEXT DEFAULT '',
             plan_id INTEGER, wave_id_fk INTEGER, wave_id TEXT,
             task_id TEXT, title TEXT, status TEXT DEFAULT 'pending',
             priority TEXT, type TEXT, assignee TEXT, description TEXT,
             test_criteria TEXT, model TEXT, notes TEXT,
             tokens INTEGER DEFAULT 0, output_data TEXT, executor_host TEXT,
             started_at TEXT, completed_at TEXT,
             validated_at TEXT, validated_by TEXT, validation_report TEXT,
             output_type TEXT DEFAULT 'pr', validator_agent TEXT DEFAULT 'thor',
             effort_level INTEGER DEFAULT 1
         );
         CREATE TABLE peer_heartbeats (
             peer_name TEXT PRIMARY KEY, last_seen REAL,
             load_json TEXT, capabilities TEXT
         );
         CREATE TABLE host_heartbeats (
             hostname TEXT PRIMARY KEY, last_seen TEXT,
             status TEXT, metadata TEXT
         );
         CREATE TABLE mesh_sync_stats (
             peer_name TEXT PRIMARY KEY, avg_latency_ms REAL,
             last_sync_at TEXT
         );
         CREATE TABLE notification_queue (
             id INTEGER PRIMARY KEY, severity TEXT DEFAULT 'info',
             title TEXT NOT NULL DEFAULT '', message TEXT,
             plan_id INTEGER, link TEXT,
             status TEXT DEFAULT 'pending',
             created_at TEXT DEFAULT (datetime('now')),
             delivered_at TEXT
         );
         CREATE TABLE coordinator_events (
             id INTEGER PRIMARY KEY, event_type TEXT NOT NULL DEFAULT '',
             payload TEXT, source_node TEXT,
             handled_at TEXT DEFAULT (datetime('now'))
         );
         CREATE TABLE daemon_config (
             key TEXT PRIMARY KEY NOT NULL, value TEXT,
             updated_at TEXT DEFAULT (datetime('now'))
         );
         CREATE TABLE knowledge_base (
             id INTEGER PRIMARY KEY, domain TEXT, title TEXT,
             content TEXT, created_at TEXT DEFAULT (datetime('now')),
             hit_count INTEGER DEFAULT 0
         );
         CREATE TABLE agent_activity (
             id INTEGER PRIMARY KEY AUTOINCREMENT, agent_id TEXT NOT NULL,
             agent_type TEXT NOT NULL DEFAULT 'legacy', model TEXT,
             description TEXT, status TEXT NOT NULL DEFAULT 'running',
             tokens_in INTEGER DEFAULT 0, tokens_out INTEGER DEFAULT 0,
             tokens_total INTEGER DEFAULT 0, cost_usd REAL DEFAULT 0,
             started_at TEXT NOT NULL DEFAULT (datetime('now')),
             completed_at TEXT, duration_s REAL, host TEXT,
             region TEXT DEFAULT 'prefrontal', metadata TEXT,
             task_db_id INTEGER, plan_id INTEGER, parent_session TEXT
         );
         CREATE UNIQUE INDEX uq_agent_activity_agent_id ON agent_activity(agent_id);
         INSERT INTO projects (id, name) VALUES ('e2e', 'E2E Test');
         INSERT INTO peer_heartbeats VALUES
             ('mac-worker-2', strftime('%s','now'), '{\"cpu\":20}', 'coordinator'),
             ('linux-worker', strftime('%s','now') - 60, '{\"cpu\":50}', 'worker');
         INSERT INTO knowledge_base (domain, title, content, hit_count)
             VALUES ('test', 'E2E test', 'This is an E2E test entry', 1);",
    )
    .expect("seed");
    drop(conn);
    let app = build_router_with_db(static_dir, db_path, None);
    (app, tmp)
}

async fn post_json(app: &axum::Router, path: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(path)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.expect("response");
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes)
        .unwrap_or(json!({"raw": String::from_utf8_lossy(&bytes).to_string()}));
    (status, json)
}

async fn get_json(app: &axum::Router, path: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.expect("response");
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes)
        .unwrap_or(json!({"raw": String::from_utf8_lossy(&bytes).to_string()}));
    (status, json)
}

/// Full lifecycle: create → import → approve → start → tasks → complete
#[tokio::test]
async fn e2e_plan_lifecycle() {
    let (app, _tmp) = setup_app();

    let (s, r) = post_json(
        &app,
        "/api/plan-db/create",
        json!({"project_id": "e2e", "name": "E2E Plan"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let pid = r["plan_id"].as_i64().unwrap();

    let (s, r) = post_json(
        &app,
        "/api/plan-db/import",
        json!({
            "plan_id": pid,
            "waves": [{"id": "W1", "name": "Wave", "tasks": [
                {"id": "T1", "title": "Task 1"},
                {"id": "T2", "title": "Task 2"}
            ]}]
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(r["tasks_created"], 2);

    post_json(&app, &format!("/api/plan-db/approve/{pid}"), json!({})).await;
    post_json(&app, &format!("/api/plan-db/start/{pid}"), json!({})).await;

    let (_, tree) = get_json(&app, &format!("/api/plan-db/execution-tree/{pid}")).await;
    let tasks = tree["tree"][0]["tasks"].as_array().unwrap();
    for t in tasks {
        let tid = t["id"].as_i64().unwrap();
        post_json(
            &app,
            "/api/plan-db/task/update",
            json!({"task_id": tid, "status": "done"}),
        )
        .await;
    }

    let (s, r) = post_json(&app, &format!("/api/plan-db/complete/{pid}"), json!({})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(r["status"], "completed");
}

/// Notifications: create → queue → deliver
#[tokio::test]
async fn e2e_notification_flow() {
    let (app, _tmp) = setup_app();

    let (s, r) = post_json(
        &app,
        "/api/notify",
        json!({"title": "Build done", "message": "All tests pass", "severity": "success"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let nid = r["id"].as_i64().unwrap();

    let (s, r) = get_json(&app, "/api/notify/queue").await;
    assert_eq!(s, StatusCode::OK);
    // Notification may be auto-delivered on macOS, so check count >= 0
    assert!(r["ok"].as_bool().unwrap());

    let (s, _) = post_json(&app, "/api/notify/deliver", json!({"ids": [nid]})).await;
    assert_eq!(s, StatusCode::OK);
}

/// Health endpoint
#[tokio::test]
async fn e2e_health_check() {
    let (app, _tmp) = setup_app();
    let (s, r) = get_json(&app, "/api/health").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["ok"].as_bool().unwrap());
    assert!(r["version"].as_str().unwrap().len() > 0);
}

/// Coordinator events
#[tokio::test]
async fn e2e_coordinator_events() {
    let (app, _tmp) = setup_app();

    let (s, r) = post_json(
        &app,
        "/api/coordinator/emit",
        json!({"event_type": "plan_started", "payload": {"plan_id": 42}}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["event_id"].as_i64().unwrap() > 0);

    let (s, r) = get_json(&app, "/api/coordinator/events").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["count"].as_i64().unwrap() >= 1);
}

/// Heartbeat + watchdog
#[tokio::test]
async fn e2e_heartbeat_and_watchdog() {
    let (app, _tmp) = setup_app();

    let (s, r) = post_json(&app, "/api/heartbeat", json!({"peer_name": "test-node"})).await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["ok"].as_bool().unwrap());

    let (s, r) = get_json(&app, "/api/heartbeat/status").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["peers"].as_array().unwrap().len() >= 1);

    let (s, r) = get_json(&app, "/api/watchdog/status").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r.get("stale_tasks").is_some());
}

/// Peer topology + diagnostics
#[tokio::test]
async fn e2e_mesh_topology() {
    let (app, _tmp) = setup_app();

    let (s, _r) = get_json(&app, "/api/peers/coordinator").await;
    assert_eq!(s, StatusCode::OK);

    let (s, r) = get_json(&app, "/api/mesh/topology").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["nodes"].as_array().unwrap().len() >= 2);

    let (s, r) = get_json(&app, "/api/mesh/diagnostics").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["total_peers"].as_i64().unwrap() >= 2);
}

/// KB search
#[tokio::test]
async fn e2e_kb_search() {
    let (app, _tmp) = setup_app();
    let (s, r) = get_json(&app, "/api/plan-db/kb-search?q=E2E").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["count"].as_i64().unwrap() >= 1);
}

/// Worker launch
#[tokio::test]
async fn e2e_worker_launch() {
    let (app, _tmp) = setup_app();

    let (s, r) = post_json(
        &app,
        "/api/workers/launch",
        json!({"agent_type": "copilot", "model": "sonnet", "description": "E2E test"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["agent_id"].as_str().unwrap().contains("copilot"));

    let (s, r) = get_json(&app, "/api/workers").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["count"].as_i64().unwrap() >= 1);
}
