// CLI-to-HTTP integration tests — agent, bus, IPC, and review endpoints.
// Split from api_cli_integration_tests.rs to keep each file under 250 lines.
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
        "claude-cli-ipc-integ-{}-{n}.db",
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

// --- Agent start/complete ---

#[tokio::test]
async fn cli_agent_start_and_complete() {
    let app = test_router();
    let (s, j) = post_json(
        &app,
        "/api/ipc/agents/register",
        json!({ "agent_id": "claude-m5max-42", "host": "m5max", "agent_type": "claude" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["agent_id"], "claude-m5max-42");

    let (_, j) = get(&app, "/api/ipc/agents").await;
    assert_eq!(j["agents"].as_array().unwrap().len(), 1);

    let (s, _) = post_json(
        &app,
        "/api/ipc/agents/unregister",
        json!({ "agent_id": "claude-m5max-42", "host": "m5max" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (_, j) = get(&app, "/api/ipc/agents").await;
    assert!(j["agents"].as_array().unwrap().is_empty());
}

// --- Bus who/send/read ---

#[tokio::test]
async fn cli_bus_who_and_send() {
    let app = test_router();
    let _ = post_json(
        &app,
        "/api/ipc/agents/register",
        json!({ "agent_id": "planner-m5max-1", "host": "m5max" }),
    )
    .await;

    let (s, j) = get(&app, "/api/ipc/agents").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["agents"].as_array().unwrap().len(), 1);

    let (s, _) = post_json(
        &app,
        "/api/ipc/send",
        json!({ "channel": "coord", "content": "T5-03 started", "sender_name": "planner" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (_, j) = get(&app, "/api/ipc/messages?channel=coord").await;
    let msgs = j["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["content"], "T5-03 started");
}

// --- IPC status ---

#[tokio::test]
async fn cli_ipc_status_overview() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/status").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["agents_active"].is_number());
    assert!(j["locks_active"].is_number());
    assert!(j["messages_total"].is_number());
}

// --- Review register/check ---

#[tokio::test]
async fn cli_review_register_and_check() {
    let app = test_router();
    let (_, j) = post_json(
        &app,
        "/api/plan-db/create",
        json!({ "project_id": "convergio", "name": "Review Test" }),
    )
    .await;
    let plan_id = j["plan_id"].as_i64().unwrap();

    let (s, _) = post_json(
        &app,
        "/api/plan-db/review/register",
        json!({
            "plan_id": plan_id,
            "reviewer_agent": "plan-reviewer",
            "verdict": "proceed",
            "suggestions": "Solid plan."
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (s, j) = get(
        &app,
        &format!("/api/plan-db/review/check?plan_id={plan_id}"),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["total"], 1);
    assert_eq!(j["reviewer"], 1);
}
