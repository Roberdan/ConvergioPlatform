// End-to-end integration tests for the workspace layer.
// Why: validates full data path across workspace CRUD, event recording/querying,
// quality gate error handling, release validation, and deliverables — all via
// a real in-memory DB through the full axum router.
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

// ── Test harness ──────────────────────────────────────────────────────────────

struct TestApp {
    router: axum::Router,
    db_path: std::path::PathBuf,
}

impl TestApp {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_path =
            std::env::temp_dir().join(format!("claude-ws-integ-{}-{n}.db", std::process::id()));
        // Dev-mode: disable auth so tests pass without a bearer token.
        super::middleware::set_dev_mode(true);
        let router = super::routes::build_router_with_db(
            std::path::PathBuf::from("/tmp"),
            db_path.clone(),
            None,
        );
        Self { router, db_path }
    }

    /// Insert a workspace row directly — bypasses git so tests run in CI without a repo.
    fn seed_workspace(&self, workspace_id: &str, plan_id: Option<i64>) {
        use r2d2::Pool;
        use r2d2_sqlite::SqliteConnectionManager;
        let pool = Pool::builder()
            .max_size(1)
            .build(SqliteConnectionManager::file(&self.db_path))
            .unwrap();
        pool.get()
            .unwrap()
            .execute(
                "INSERT OR IGNORE INTO workspaces \
                 (plan_id, workspace_id, path, branch, status) \
                 VALUES (?1, ?2, ?3, ?4, 'active')",
                rusqlite::params![
                    plan_id,
                    workspace_id,
                    format!("/tmp/{workspace_id}"),
                    format!("workspace/{workspace_id}")
                ],
            )
            .unwrap();
    }
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

// 1. GET /api/workspace/list on a fresh DB → ok + empty array
#[tokio::test]
async fn test_workspace_list_empty() {
    let app = TestApp::new();
    let resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/list")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["ok"], true);
    let arr = body["workspaces"].as_array().unwrap();
    assert!(arr.is_empty(), "fresh DB must return no workspaces");
}

// 2. POST /api/workspace/events/record + GET /api/workspace/events roundtrip
#[tokio::test]
async fn test_record_and_query_events() {
    let app = TestApp::new();
    app.seed_workspace("ws-integ-ev-01", None);

    let record_payload = serde_json::json!({
        "workspace_id": "ws-integ-ev-01",
        "agent":        "task-executor",
        "action":       "file_write",
        "file_path":    "src/main.rs",
        "detail":       "added 20 lines"
    })
    .to_string();

    // Record event
    let post_resp = app
        .router
        .clone()
        .oneshot(
            Request::post("/api/workspace/events/record")
                .header("content-type", "application/json")
                .body(Body::from(record_payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(post_resp.status(), StatusCode::OK);
    let post_body = body_json(post_resp).await;
    assert_eq!(post_body["ok"], true);
    assert!(
        post_body["event_id"]
            .as_i64()
            .map(|n| n > 0)
            .unwrap_or(false),
        "event_id must be a positive integer"
    );

    // Query events — recorded event must appear
    let get_resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/events?workspace_id=ws-integ-ev-01")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let get_body = body_json(get_resp).await;
    let events = get_body["events"].as_array().unwrap();
    assert!(
        !events.is_empty(),
        "events array must contain the recorded event"
    );
    assert!(
        events
            .iter()
            .any(|e| e["action"] == "file_write" && e["file_path"] == "src/main.rs"),
        "recorded event must appear in query results"
    );
}

// 3. POST /api/workspace/quality-gate on a non-existent workspace → 404
#[tokio::test]
async fn test_quality_gate_endpoint_nonexistent_workspace() {
    let app = TestApp::new();
    let payload = serde_json::json!({"workspace_id": "ws-ghost-9999"}).to_string();
    let resp = app
        .router
        .oneshot(
            Request::post("/api/workspace/quality-gate")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "quality-gate on missing workspace must return 404"
    );
}

#[path = "api_workspace_integration_tests_lifecycle.rs"]
mod lifecycle;
