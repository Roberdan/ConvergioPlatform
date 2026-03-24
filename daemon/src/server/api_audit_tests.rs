// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for GET /api/audit/project/:project_id endpoint.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router() -> (axum::Router, std::path::PathBuf) {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!("claude-audit-test-{}-{n}.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    drop(conn);
    super::middleware::set_dev_mode(true);
    let router =
        super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp.clone(), None);
    (router, tmp)
}

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS plans (
  id INTEGER PRIMARY KEY, name TEXT, status TEXT,
  project_id TEXT, tasks_total INTEGER DEFAULT 0,
  tasks_done INTEGER DEFAULT 0, created_at TEXT, updated_at TEXT
);
CREATE TABLE IF NOT EXISTS tasks (
  id INTEGER PRIMARY KEY, plan_id INTEGER, task_id TEXT,
  title TEXT, status TEXT, model TEXT, effort TEXT, wave_id_fk INTEGER
);
CREATE TABLE IF NOT EXISTS execution_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT, goal TEXT NOT NULL,
  status TEXT DEFAULT 'running', cost_usd REAL DEFAULT 0,
  duration_minutes REAL, agents_used INTEGER DEFAULT 0,
  plan_id INTEGER, started_at TEXT DEFAULT (datetime('now')),
  completed_at TEXT
);
CREATE TABLE IF NOT EXISTS knowledge_base (
  id INTEGER PRIMARY KEY, domain TEXT, title TEXT,
  content TEXT, created_at TEXT
);
";

const SEED: &str = "
INSERT INTO plans(id,name,status,project_id,tasks_total,tasks_done,created_at,updated_at)
  VALUES(1,'Security Hardening','doing','proj-alpha',3,2,'2026-03-20','2026-03-22');
INSERT INTO tasks(id,plan_id,task_id,title,status,model,effort,wave_id_fk)
  VALUES(10,1,'W1-T1','Input validation','done','opus','M',NULL);
INSERT INTO tasks(id,plan_id,task_id,title,status,model,effort,wave_id_fk)
  VALUES(11,1,'W1-T2','Rate limiting','done','sonnet','S',NULL);
INSERT INTO tasks(id,plan_id,task_id,title,status,model,effort,wave_id_fk)
  VALUES(12,1,'W1-T3','Auth hardening','todo','opus','L',NULL);
INSERT INTO knowledge_base(id,domain,title,content,created_at)
  VALUES(1,'proj-alpha','OWASP Top 10','Injection prevention','2026-03-21');
";

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 131072).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn seed_db(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).expect("open for seed");
    // Ensure columns the audit endpoint queries but init may not create
    let _ = conn.execute_batch("ALTER TABLE agent_activity ADD COLUMN action TEXT DEFAULT ''");
    conn.execute_batch(SEED).expect("seed data");
}

// --- GET /api/audit/project/:project_id (with data) -----------------------

#[tokio::test]
async fn audit_returns_full_report_shape() {
    let (app, db) = test_router();
    seed_db(&db);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/audit/project/proj-alpha")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;

    // Top-level fields
    assert_eq!(json["project_id"], "proj-alpha");
    assert!(json["generated_at"].is_string());
    assert!(json["summary"].is_object());
    assert!(json["plans"].is_array());
    assert!(json["tasks"].is_array());
    assert!(json["solve_sessions"].is_array());
    assert!(json["execution_runs"].is_array());
    assert!(json["agent_activity"].is_array());
    assert!(json["deliverables"].is_array());
    assert!(json["kb_learnings"].is_array());
}

#[tokio::test]
async fn audit_summary_counts_match_seeded_data() {
    let (app, db) = test_router();
    seed_db(&db);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/audit/project/proj-alpha")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    let summary = &json["summary"];
    assert_eq!(summary["plans"], 1);
    assert_eq!(summary["tasks_total"], 3);
    assert_eq!(summary["tasks_done"], 2);
    assert_eq!(summary["kb_learnings"], 1);
}

#[tokio::test]
async fn audit_plans_contain_expected_fields() {
    let (app, db) = test_router();
    seed_db(&db);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/audit/project/proj-alpha")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    let plans = json["plans"].as_array().unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0]["name"], "Security Hardening");
    assert_eq!(plans[0]["status"], "doing");
}

#[tokio::test]
async fn audit_tasks_contain_model_and_status() {
    let (app, db) = test_router();
    seed_db(&db);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/audit/project/proj-alpha")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    let tasks = json["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 3);
    let done_tasks: Vec<_> = tasks.iter().filter(|t| t["status"] == "done").collect();
    assert_eq!(done_tasks.len(), 2);
}

// --- GET /api/audit/project/:project_id (empty project) -------------------

#[tokio::test]
async fn audit_unknown_project_returns_empty_report() {
    let (app, _db) = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/audit/project/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["project_id"], "nonexistent");
    let s = &json["summary"];
    assert_eq!(s["plans"], 0);
    assert_eq!(s["tasks_total"], 0);
    assert_eq!(s["tasks_done"], 0);
    assert_eq!(s["total_cost_usd"], 0.0);
}

#[tokio::test]
async fn audit_kb_learnings_match_by_domain() {
    let (app, db) = test_router();
    seed_db(&db);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/audit/project/proj-alpha")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    let kb = json["kb_learnings"].as_array().unwrap();
    assert_eq!(kb.len(), 1);
    assert_eq!(kb[0]["title"], "OWASP Top 10");
}
