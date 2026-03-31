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
        // Dev-mode: disable auth so tests pass without a bearer token.
        super::middleware::set_dev_mode(true);
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

    fn seed_plan(&self, plan_id: i64) {
        use r2d2::Pool;
        use r2d2_sqlite::SqliteConnectionManager;
        let pool = Pool::builder()
            .max_size(1)
            .build(SqliteConnectionManager::file(&self.db_path))
            .unwrap();
        pool.get()
            .unwrap()
            .execute(
                "INSERT OR IGNORE INTO plans (id, project_id, name, status) VALUES (?1, 'test', ?2, 'doing')",
                rusqlite::params![plan_id, format!("plan-{plan_id}")],
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

#[tokio::test]
async fn bind_workspace_context_updates_plan_execution_fields() {
    let app = TestApp::new();
    app.seed_plan(42);
    use r2d2::Pool;
    use r2d2_sqlite::SqliteConnectionManager;
    let pool = Pool::builder()
        .max_size(1)
        .build(SqliteConnectionManager::file(&app.db_path))
        .unwrap();
    let conn = pool.get().unwrap();
    let ws = crate::workspace::core::WorkspaceInfo {
        workspace_id: "ws-bind-42".into(),
        path: "/tmp/ws-bind-42".into(),
        branch: Some("workspace/ws-bind-42".into()),
        plan_id: Some(42),
        wave_db_id: None,
        status: "active".into(),
        created_at: "now".into(),
    };
    super::api_workspace_support::bind_workspace_context(&conn, &ws).unwrap();
    let (worktree_path, branch_name): (String, String) = conn
        .query_row(
            "SELECT COALESCE(worktree_path, ''), COALESCE(branch_name, '') FROM plans WHERE id = 42",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(worktree_path, ws.path);
    assert_eq!(branch_name, ws.branch.unwrap());
}

#[path = "api_workspace_tests_events.rs"]
mod events_tests;
