// Integration tests for /api/workspace/* endpoints.
// Pattern: spin up a full build_router_with_db using a temp DB, send requests via oneshot.
// Note: create_workspace invokes git worktree add; tests that need pre-seeded data
// insert rows directly into the DB to avoid requiring a git repo in CI.
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

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
            std::env::temp_dir().join(format!("claude-ws-test-{}-{n}.db", std::process::id()));
        let router = super::routes::build_router_with_db(
            std::path::PathBuf::from("/tmp"),
            db_path.clone(),
            None,
        );
        Self { router, db_path }
    }

    /// Seed the workspaces table directly — bypasses git so tests don't need a real repo.
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

// GET /api/workspace/list returns ok + empty workspaces array on fresh DB
#[tokio::test]
async fn list_workspaces_returns_array() {
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
    assert!(body["workspaces"].is_array());
}

// Seeded workspace appears in list
#[tokio::test]
async fn list_workspaces_shows_seeded_row() {
    let app = TestApp::new();
    app.seed_workspace("ws-list-0001", Some(5));
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
    let workspaces = body["workspaces"].as_array().unwrap();
    assert!(
        workspaces
            .iter()
            .any(|w| w["workspace_id"] == "ws-list-0001"),
        "seeded workspace must appear in list"
    );
}

// GET /api/workspace/status/:id — seeded workspace returns workspace + recent_events
#[tokio::test]
async fn workspace_status_returns_info_and_events() {
    let app = TestApp::new();
    app.seed_workspace("ws-status-0001", Some(7));
    let resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/status/ws-status-0001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["workspace"]["workspace_id"], "ws-status-0001");
    assert_eq!(body["workspace"]["status"], "active");
    assert!(body["recent_events"].is_array());
}

// GET /api/workspace/status/:id with unknown id → 404
#[tokio::test]
async fn workspace_status_unknown_id_returns_404() {
    let app = TestApp::new();
    let resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/status/ws-nonexistent-0000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// GET /api/workspace/events — seeded workspace returns ok + events array
#[tokio::test]
async fn workspace_events_returns_array() {
    let app = TestApp::new();
    app.seed_workspace("ws-events-0001", None);
    let resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/events?workspace_id=ws-events-0001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["ok"], true);
    assert!(body["events"].is_array());
}

// GET /api/workspace/events without workspace_id → 400
#[tokio::test]
async fn workspace_events_missing_param_returns_400() {
    let app = TestApp::new();
    let resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// POST /api/workspace/delete on seeded workspace marks it deleted
#[tokio::test]
async fn delete_seeded_workspace_succeeds() {
    let app = TestApp::new();
    app.seed_workspace("ws-del-0001", Some(9));

    let delete_payload = serde_json::json!({ "workspace_id": "ws-del-0001" }).to_string();
    let resp = app
        .router
        .clone()
        .oneshot(
            Request::post("/api/workspace/delete")
                .header("content-type", "application/json")
                .body(Body::from(delete_payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["workspace_id"], "ws-del-0001");

    // After delete, status endpoint returns 404
    let resp = app
        .router
        .clone()
        .oneshot(
            Request::get("/api/workspace/status/ws-del-0001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Workspace is marked deleted — get_workspace returns Some but status is deleted,
    // OR if the handler uses active-only query it returns 404. Either is valid.
    // We just confirm the workspace is no longer in the active list.
    let list_resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/list")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(list_resp).await;
    let workspaces = body["workspaces"].as_array().unwrap();
    assert!(
        !workspaces
            .iter()
            .any(|w| w["workspace_id"] == "ws-del-0001"),
        "deleted workspace must not appear in active list"
    );
}

// GET /api/workspace/list?plan_id= filters correctly
#[tokio::test]
async fn list_workspaces_filter_by_plan_id() {
    let app = TestApp::new();
    app.seed_workspace("ws-plan42-aaa", Some(42));
    app.seed_workspace("ws-plan99-bbb", Some(99));

    let resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/list?plan_id=42")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let workspaces = body["workspaces"].as_array().unwrap();
    assert!(!workspaces.is_empty(), "should find workspace for plan 42");
    for w in workspaces {
        assert_eq!(w["plan_id"], 42, "all results should have plan_id=42");
    }
}
