// IPC HTTP endpoint integration tests — coordination layer (Plan 742 T5-03)
// Tests: agents CRUD, messages, channels, context, locks, worktrees, conflicts, status, send
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-ipc-integ-{}-{n}.db",
        std::process::id()
    ));
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
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

// --- Messages: send → list → filter by channel ---

#[tokio::test]
async fn ipc_send_and_list_messages() {
    let app = test_router();
    let (s, _) = post_json(
        &app,
        "/api/ipc/send",
        json!({ "channel": "ops", "content": "plan 742 started", "sender_name": "ali-orchestrator" }),
    ).await;
    assert_eq!(s, StatusCode::OK);

    let (s, j) = get(&app, "/api/ipc/messages?channel=ops&limit=10").await;
    assert_eq!(s, StatusCode::OK);
    let msgs = j["messages"].as_array().unwrap();
    assert!(!msgs.is_empty(), "should have at least one message");
    assert_eq!(msgs[0]["channel"], "ops");
    assert_eq!(msgs[0]["from_agent"], "ali-orchestrator");
}

#[tokio::test]
async fn ipc_messages_default_channel_is_general() {
    let app = test_router();
    let _ = post_json(
        &app,
        "/api/ipc/send",
        json!({ "content": "hello mesh", "sender_name": "dario-debugger" }),
    ).await;

    let (_, j) = get(&app, "/api/ipc/messages?channel=general").await;
    let msgs = j["messages"].as_array().unwrap();
    assert!(!msgs.is_empty(), "message with no channel goes to general");
}

#[tokio::test]
async fn ipc_messages_limit_respected() {
    let app = test_router();
    for i in 0..5 {
        let _ = post_json(
            &app,
            "/api/ipc/send",
            json!({ "channel": "flood", "content": format!("msg-{i}"), "sender_name": "bot" }),
        ).await;
    }

    let (_, j) = get(&app, "/api/ipc/messages?channel=flood&limit=2").await;
    let msgs = j["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 2, "limit=2 should return exactly 2 messages");
}

// --- Channels ---

#[tokio::test]
async fn ipc_channels_returns_ok_empty() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/channels").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["channels"].is_array());
}

#[tokio::test]
async fn ipc_send_message_appears_in_channel_query() {
    let app = test_router();
    let _ = post_json(
        &app,
        "/api/ipc/send",
        json!({ "channel": "mesh-alerts", "content": "peer down", "sender_name": "monitor" }),
    ).await;

    let (s, j) = get(&app, "/api/ipc/messages?channel=mesh-alerts").await;
    assert_eq!(s, StatusCode::OK);
    let msgs = j["messages"].as_array().unwrap();
    assert!(!msgs.is_empty(), "message should be queryable by channel");
    assert_eq!(msgs[0]["content"], "peer down");
}

// --- Status aggregates ---

#[tokio::test]
async fn ipc_status_reflects_registered_agents_and_messages() {
    let app = test_router();
    let _ = post_json(
        &app,
        "/api/ipc/agents/register",
        json!({ "agent_id": "a1", "host": "h1" }),
    ).await;
    let _ = post_json(
        &app,
        "/api/ipc/agents/register",
        json!({ "agent_id": "a2", "host": "h1" }),
    ).await;
    let _ = post_json(
        &app,
        "/api/ipc/send",
        json!({ "content": "test", "sender_name": "a1" }),
    ).await;

    let (s, j) = get(&app, "/api/ipc/status").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["agents_active"], 2);
    assert!(j["messages_total"].as_i64().unwrap() >= 1);
    assert_eq!(j["locks_active"], 0);
    assert_eq!(j["conflicts"], 0);
}

// --- Context (shared) ---

#[tokio::test]
async fn ipc_context_empty_initially() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/context").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["context"].as_array().unwrap().is_empty());
}

// --- Locks ---

#[tokio::test]
async fn ipc_locks_empty_initially() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/locks").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["locks"].as_array().unwrap().is_empty());
}

// --- Worktrees ---

#[tokio::test]
async fn ipc_worktrees_empty_initially() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/worktrees").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["worktrees"].as_array().unwrap().is_empty());
}

// --- Conflicts ---

#[tokio::test]
async fn ipc_conflicts_empty_with_no_locks() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/conflicts").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["conflicts"].as_array().unwrap().is_empty());
}
