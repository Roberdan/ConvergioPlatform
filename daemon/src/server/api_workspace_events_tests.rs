// Integration tests for POST /api/workspace/events/record endpoint.
// Pattern: spin up full build_router_with_db with temp DB, send requests via oneshot.
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
        let db_path = std::env::temp_dir().join(format!(
            "claude-ws-events-test-{}-{n}.db",
            std::process::id()
        ));
        let router = super::routes::build_router_with_db(
            std::path::PathBuf::from("/tmp"),
            db_path.clone(),
            None,
        );
        Self { router, db_path }
    }

    fn seed_workspace(&self, workspace_id: &str) {
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
                 VALUES (NULL, ?1, ?2, ?3, 'active')",
                rusqlite::params![
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

// POST /api/workspace/events/record with valid payload → 200 ok + event_id
#[tokio::test]
async fn record_event_returns_ok_and_event_id() {
    let app = TestApp::new();
    app.seed_workspace("ws-rec-0001");

    let payload = serde_json::json!({
        "workspace_id": "ws-rec-0001",
        "agent": "task-executor",
        "action": "file_write",
        "file_path": "daemon/src/main.rs"
    })
    .to_string();

    let resp = app
        .router
        .oneshot(
            Request::post("/api/workspace/events/record")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["ok"], true);
    assert!(
        body["event_id"].as_i64().map(|n| n > 0).unwrap_or(false),
        "event_id must be a positive integer"
    );
}

// Record event then GET /api/workspace/events confirms it is stored
#[tokio::test]
async fn record_event_then_get_events_shows_recorded() {
    let app = TestApp::new();
    app.seed_workspace("ws-rec-0002");

    let payload = serde_json::json!({
        "workspace_id": "ws-rec-0002",
        "agent": "workspace-hook",
        "action": "file_edit",
        "file_path": "src/lib.rs",
        "detail": "edited 10 lines"
    })
    .to_string();

    // Record the event
    let post_resp = app
        .router
        .clone()
        .oneshot(
            Request::post("/api/workspace/events/record")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(post_resp.status(), StatusCode::OK);

    // Retrieve events — must include the one we just recorded
    let get_resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/events?workspace_id=ws-rec-0002")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let body = body_json(get_resp).await;
    let events = body["events"].as_array().unwrap();
    assert!(!events.is_empty(), "at least one event must be present");
    let ev = events.iter().find(|e| e["agent"] == "workspace-hook");
    assert!(
        ev.is_some(),
        "recorded event must appear in GET /api/workspace/events"
    );
    let ev = ev.unwrap();
    assert_eq!(ev["action"], "file_edit");
    assert_eq!(ev["file_path"], "src/lib.rs");
    assert_eq!(ev["detail"], "edited 10 lines");
}

// Missing required fields → 422 Unprocessable Entity
#[tokio::test]
async fn record_event_missing_workspace_id_returns_422() {
    let app = TestApp::new();

    let payload = serde_json::json!({
        "agent": "task-executor",
        "action": "file_write"
    })
    .to_string();

    let resp = app
        .router
        .oneshot(
            Request::post("/api/workspace/events/record")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// Unknown action string is forwarded as-is (extensible log)
#[tokio::test]
async fn record_event_custom_action_string_stored() {
    let app = TestApp::new();
    app.seed_workspace("ws-rec-0003");

    let payload = serde_json::json!({
        "workspace_id": "ws-rec-0003",
        "agent": "custom-agent",
        "action": "custom_action_xyz",
    })
    .to_string();

    let resp = app
        .router
        .clone()
        .oneshot(
            Request::post("/api/workspace/events/record")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["ok"], true);
}
