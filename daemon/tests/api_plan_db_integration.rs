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
    // Pre-create required tables
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
             position INTEGER DEFAULT 0, depends_on TEXT, estimated_hours INTEGER DEFAULT 8,
             worktree_path TEXT, started_at TEXT, completed_at TEXT,
             cancelled_at TEXT, cancelled_reason TEXT, merge_mode TEXT DEFAULT 'sync',
             theme TEXT
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
         CREATE UNIQUE INDEX IF NOT EXISTS uq_agent_activity_agent_id ON agent_activity(agent_id);
         INSERT INTO projects (id, name) VALUES ('test', 'Test Project');
         INSERT INTO knowledge_base (domain, title, content, hit_count)
             VALUES ('rust', 'Axum routing', 'Use Router::new()', 3);",
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
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body)
        .unwrap_or(json!({"raw": String::from_utf8_lossy(&body).to_string()}));
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
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body)
        .unwrap_or(json!({"raw": String::from_utf8_lossy(&body).to_string()}));
    (status, json)
}

#[tokio::test]
async fn api_plan_db_integration_full_lifecycle() {
    let (app, _tmp) = setup_app();

    // 1. Create plan
    let (status, resp) = post_json(
        &app,
        "/api/plan-db/create",
        json!({"project_id": "test", "name": "Integration Test Plan"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["ok"], true);
    let plan_id = resp["plan_id"].as_i64().expect("plan_id");

    // 2. Import waves+tasks
    let (status, resp) = post_json(
        &app,
        "/api/plan-db/import",
        json!({
            "plan_id": plan_id,
            "waves": [
                {
                    "id": "W1", "name": "Wave 1",
                    "tasks": [
                        {"id": "T1-01", "title": "First task", "priority": "P0"},
                        {"id": "T1-02", "title": "Second task", "priority": "P1"}
                    ]
                }
            ]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["waves_created"], 1);
    assert_eq!(resp["tasks_created"], 2);

    // 3. Approve
    let (status, resp) =
        post_json(&app, &format!("/api/plan-db/approve/{plan_id}"), json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["status"], "approved");

    // 4. Start
    let (status, resp) = post_json(&app, &format!("/api/plan-db/start/{plan_id}"), json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["status"], "doing");

    // 5. Update task to in_progress then done
    // Find task IDs first
    let (status, resp) = get_json(&app, &format!("/api/plan-db/execution-tree/{plan_id}")).await;
    assert_eq!(status, StatusCode::OK);
    let tree = resp["tree"].as_array().expect("tree");
    assert_eq!(tree.len(), 1);
    let tasks = tree[0]["tasks"].as_array().expect("tasks");
    assert_eq!(tasks.len(), 2);
    let task1_id = tasks[0]["id"].as_i64().expect("task id");
    let task2_id = tasks[1]["id"].as_i64().expect("task id");

    // Update task 1 to done
    let (status, _) = post_json(
        &app,
        "/api/plan-db/task/update",
        json!({"task_id": task1_id, "status": "done", "notes": "completed"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Update task 2 to done
    let (status, _) = post_json(
        &app,
        "/api/plan-db/task/update",
        json!({"task_id": task2_id, "status": "done"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 6. List plans — should show our plan
    let (status, resp) = get_json(&app, "/api/plan-db/list").await;
    assert_eq!(status, StatusCode::OK);
    let plans = resp["plans"].as_array().expect("plans");
    assert!(!plans.is_empty());

    // 7. Complete plan
    let (status, resp) =
        post_json(&app, &format!("/api/plan-db/complete/{plan_id}"), json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["status"], "completed");
}

#[tokio::test]
async fn api_plan_db_integration_kb_search() {
    let (app, _tmp) = setup_app();

    let (status, resp) = get_json(&app, "/api/plan-db/kb-search?q=axum").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["ok"], true);
    assert!(resp["count"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn api_plan_db_integration_cancel() {
    let (app, _tmp) = setup_app();

    let (_, resp) = post_json(
        &app,
        "/api/plan-db/create",
        json!({"project_id": "test", "name": "Cancel Test"}),
    )
    .await;
    let plan_id = resp["plan_id"].as_i64().unwrap();

    // Start it
    post_json(&app, &format!("/api/plan-db/start/{plan_id}"), json!({})).await;

    // Cancel
    let (status, resp) = post_json(
        &app,
        &format!("/api/plan-db/cancel/{plan_id}"),
        json!({"reason": "no longer needed"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["status"], "cancelled");
}

#[tokio::test]
async fn api_plan_db_integration_complete_blocks_with_pending() {
    let (app, _tmp) = setup_app();

    let (_, resp) = post_json(
        &app,
        "/api/plan-db/create",
        json!({"project_id": "test", "name": "Block Test"}),
    )
    .await;
    let plan_id = resp["plan_id"].as_i64().unwrap();

    // Import tasks
    post_json(
        &app,
        "/api/plan-db/import",
        json!({
            "plan_id": plan_id,
            "waves": [{"id": "W1", "name": "Wave 1", "tasks": [
                {"id": "T1", "title": "Pending task"}
            ]}]
        }),
    )
    .await;

    // Start
    post_json(&app, &format!("/api/plan-db/start/{plan_id}"), json!({})).await;

    // Try to complete — should fail due to pending tasks
    let (status, resp) =
        post_json(&app, &format!("/api/plan-db/complete/{plan_id}"), json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["ok"], false);
}
