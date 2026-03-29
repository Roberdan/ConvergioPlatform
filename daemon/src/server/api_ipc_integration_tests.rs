// IPC HTTP endpoint integration tests — agent lifecycle (Plan 742 T5-03)
// Tests: agents CRUD (register, list, heartbeat, unregister, upsert)
// Channel/message/status/context/lock/worktree/conflict tests → api_ipc_integration_tests2.rs
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

pub(super) fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-ipc-integ-{}-{n}.db",
        std::process::id()
    ));
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

pub(super) async fn get(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
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

pub(super) async fn post_json(router: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
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

// --- Agent register → list → heartbeat → unregister lifecycle ---

#[tokio::test]
async fn ipc_agent_register_and_list() {
    let app = test_router();
    let (s, j) = post_json(
        &app,
        "/api/ipc/agents/register",
        json!({ "agent_id": "planner-m5max-1234", "host": "m5max", "agent_type": "claude", "pid": 1234 }),
    ).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["agent_id"], "planner-m5max-1234");

    let (s, j) = get(&app, "/api/ipc/agents").await;
    assert_eq!(s, StatusCode::OK);
    let agents = j["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0]["name"], "planner-m5max-1234");
    assert_eq!(agents[0]["host"], "m5max");
    assert_eq!(agents[0]["agent_type"], "claude");
}

#[tokio::test]
async fn ipc_agent_heartbeat_updates_last_seen() {
    let app = test_router();
    let _ = post_json(
        &app,
        "/api/ipc/agents/register",
        json!({ "agent_id": "executor-m1pro-5678", "host": "m1pro" }),
    ).await;

    let (s, j) = post_json(
        &app,
        "/api/ipc/agents/heartbeat",
        json!({ "agent_id": "executor-m1pro-5678", "host": "m1pro", "current_task": "T5-03" }),
    ).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);

    let (_, j) = get(&app, "/api/ipc/agents").await;
    let agents = j["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 1);
    let meta: Value = serde_json::from_str(agents[0]["metadata"].as_str().unwrap()).unwrap();
    assert_eq!(meta["current_task"], "T5-03");
}

#[tokio::test]
async fn ipc_agent_unregister_removes_agent() {
    let app = test_router();
    let _ = post_json(
        &app,
        "/api/ipc/agents/register",
        json!({ "agent_id": "thor-m5max-9999", "host": "m5max" }),
    ).await;

    let (s, _) = post_json(
        &app,
        "/api/ipc/agents/unregister",
        json!({ "agent_id": "thor-m5max-9999", "host": "m5max" }),
    ).await;
    assert_eq!(s, StatusCode::OK);

    let (_, j) = get(&app, "/api/ipc/agents").await;
    assert!(j["agents"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn ipc_agent_register_replaces_on_same_name_host() {
    let app = test_router();
    let _ = post_json(
        &app,
        "/api/ipc/agents/register",
        json!({ "agent_id": "ali-m5max-100", "host": "m5max", "pid": 100 }),
    ).await;
    let _ = post_json(
        &app,
        "/api/ipc/agents/register",
        json!({ "agent_id": "ali-m5max-100", "host": "m5max", "pid": 200 }),
    ).await;

    let (_, j) = get(&app, "/api/ipc/agents").await;
    let agents = j["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 1, "re-register should upsert, not duplicate");
    assert_eq!(agents[0]["pid"], 200);
}
