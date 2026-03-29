// IPC HTTP endpoint integration tests — messages, channels, status, context, locks, worktrees, conflicts.
// Agent lifecycle tests → api_ipc_integration_tests.rs

use serde_json::json;
use axum::http::StatusCode;

fn test_router() -> axum::Router {
    super::api_ipc_integration_tests::test_router()
}

async fn get(router: &axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    super::api_ipc_integration_tests::get(router, uri).await
}

async fn post_json(router: &axum::Router, uri: &str, payload: serde_json::Value) -> (StatusCode, serde_json::Value) {
    super::api_ipc_integration_tests::post_json(router, uri, payload).await
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
