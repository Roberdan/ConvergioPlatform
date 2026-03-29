// CLI-to-HTTP integration tests (Plan 742 T5-03)
// Exercises the HTTP endpoints that cvg subcommands call.
// Tests: plan list/show/create/start/cancel, task update, agent start/complete,
//        review register/check, IPC bus who/send.
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

const PROJECT_SEED: &str =
    "INSERT INTO projects (id, name, path) VALUES ('convergio', 'ConvergioPlatform', '/tmp/cvg');";

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-cli-integ-{}-{n}.db",
        std::process::id()
    ));
    super::middleware::set_dev_mode(true);
    let router = super::routes::build_router_with_db(
        std::path::PathBuf::from("/tmp"),
        tmp.clone(),
        None,
    );
    let conn = rusqlite::Connection::open(&tmp).expect("open seed");
    conn.execute_batch(PROJECT_SEED).expect("seed project");
    drop(conn);
    router
}

async fn get(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let resp = router
        .clone()
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

async fn post_json(router: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
    let resp = router
        .clone()
        .oneshot(
            Request::post(uri)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

/// Helper: create plan + import wave + register review + start.
/// Returns (plan_id, task_db_id).
async fn create_started_plan(app: &axum::Router) -> (i64, i64) {
    let (s, j) = post_json(
        app,
        "/api/plan-db/create",
        json!({ "project_id": "convergio", "name": "Integration Test Plan" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "create failed: {j}");
    let plan_id = j["plan_id"].as_i64().unwrap();

    let (s, j) = post_json(
        app,
        "/api/plan-db/import",
        json!({
            "plan_id": plan_id,
            "waves": [{ "id": "W1", "name": "Wave 1", "tasks": [{
                "id": "T1-01", "title": "Test task", "model": "claude-opus-4-6"
            }]}]
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "import failed: {j}");

    let (s, j) = post_json(
        app,
        "/api/plan-db/review/register",
        json!({
            "plan_id": plan_id,
            "reviewer_agent": "plan-reviewer",
            "verdict": "proceed"
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "review failed: {j}");

    let (s, j) = post_json(app, &format!("/api/plan-db/start/{plan_id}"), json!({})).await;
    assert_eq!(s, StatusCode::OK, "start failed: {j}");

    let (_, tree) = get(app, &format!("/api/plan-db/execution-tree/{plan_id}")).await;
    let task_db_id = tree["tree"][0]["tasks"][0]["id"].as_i64().unwrap();
    (plan_id, task_db_id)
}

// --- Plan list ---

#[tokio::test]
async fn cli_plan_list_returns_ok() {
    let app = test_router();
    let (s, j) = get(&app, "/api/plan-db/list").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["plans"].is_array());
}

// --- Plan create + show ---

#[tokio::test]
async fn cli_plan_create_and_show() {
    let app = test_router();
    let (s, j) = post_json(
        &app,
        "/api/plan-db/create",
        json!({ "project_id": "convergio", "name": "Hardening Plan" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let plan_id = j["plan_id"].as_i64().unwrap();

    let (s, j) = get(&app, &format!("/api/plan-db/json/{plan_id}")).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["plan"]["name"], "Hardening Plan");
    assert_eq!(j["plan"]["status"], "draft");
}

// --- Plan start (requires import + review) ---

#[tokio::test]
async fn cli_plan_start_transitions_to_doing() {
    let app = test_router();
    let (plan_id, _) = create_started_plan(&app).await;

    let (_, j) = get(&app, &format!("/api/plan-db/json/{plan_id}")).await;
    assert_eq!(j["plan"]["status"], "doing");
}

// --- Plan cancel ---

#[tokio::test]
async fn cli_plan_cancel_transitions_to_cancelled() {
    let app = test_router();
    let (_, j) = post_json(
        &app,
        "/api/plan-db/create",
        json!({ "project_id": "convergio", "name": "Cancel Test" }),
    )
    .await;
    let plan_id = j["plan_id"].as_i64().unwrap();

    let (s, _) = post_json(
        &app,
        &format!("/api/plan-db/cancel/{plan_id}"),
        json!({ "reason": "out of scope" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (_, j) = get(&app, &format!("/api/plan-db/json/{plan_id}")).await;
    assert_eq!(j["plan"]["status"], "cancelled");
}

// --- Plan show nonexistent ---

#[tokio::test]
async fn cli_plan_show_nonexistent_returns_error() {
    let app = test_router();
    let (s, _) = get(&app, "/api/plan-db/json/99999").await;
    assert!(s == StatusCode::NOT_FOUND || s == StatusCode::BAD_REQUEST);
}

// --- Task update ---

#[tokio::test]
async fn cli_task_update_changes_status() {
    let app = test_router();
    let (plan_id, task_db_id) = create_started_plan(&app).await;

    let (s, _) = post_json(
        &app,
        "/api/plan-db/task/update",
        json!({ "task_id": task_db_id, "status": "in_progress" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (_, tree) = get(&app, &format!("/api/plan-db/execution-tree/{plan_id}")).await;
    assert_eq!(tree["tree"][0]["tasks"][0]["status"], "in_progress");
}

// Agent, bus, IPC, review tests → api_cli_integration_tests_ipc.rs
