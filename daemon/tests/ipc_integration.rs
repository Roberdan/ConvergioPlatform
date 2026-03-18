// Plan 633/634: Socket-based IPC integration tests
use std::sync::Arc;

use claude_core::ipc::engine::IpcEngine;
use claude_core::ipc::protocol::{
    decode_response, encode_request, read_ipc_frame, write_ipc_frame, IpcRequest, IpcResponse,
};
use claude_core::ipc::socket::start_ipc_server;

async fn ipc_request(socket_path: &std::path::Path, request: &IpcRequest) -> IpcResponse {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let stream = tokio::net::UnixStream::connect(socket_path)
        .await
        .expect("connect to IPC socket");
    let (mut reader, mut writer) = stream.into_split();

    let req_bytes = encode_request(request).unwrap();
    write_ipc_frame(&mut writer, &req_bytes).await.unwrap();

    let resp_frame = read_ipc_frame(&mut reader).await.unwrap();
    decode_response(&resp_frame).unwrap()
}

#[tokio::test]
async fn test_round_trip_register_who() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("ipc-test.db");
    let socket_path = dir.path().join("ipc.sock");

    let engine = Arc::new(IpcEngine::new(db_path));
    let sock = socket_path.clone();
    tokio::spawn(async move {
        start_ipc_server(engine, sock).await.ok();
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Register
    let resp = ipc_request(
        &socket_path,
        &IpcRequest::Register {
            name: "test-agent".into(),
            agent_type: "claude".into(),
            pid: Some(std::process::id()),
            host: "test-host".into(),
            metadata: None,
        },
    )
    .await;
    assert!(matches!(resp, IpcResponse::Ok { .. }));

    // Who
    let resp = ipc_request(&socket_path, &IpcRequest::Who).await;
    match resp {
        IpcResponse::AgentList { agents } => {
            assert_eq!(agents.len(), 1);
            assert_eq!(agents[0].name, "test-agent");
        }
        _ => panic!("expected AgentList, got {resp:?}"),
    }
}

#[tokio::test]
async fn test_round_trip_send_receive() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("ipc-test.db");
    let socket_path = dir.path().join("ipc.sock");

    let engine = Arc::new(IpcEngine::new(db_path));
    let sock = socket_path.clone();
    tokio::spawn(async move {
        start_ipc_server(engine, sock).await.ok();
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Send
    let resp = ipc_request(
        &socket_path,
        &IpcRequest::Send {
            from: "alice".into(),
            to: "bob".into(),
            content: "hello via socket".into(),
            msg_type: "text".into(),
            priority: 0,
        },
    )
    .await;
    assert!(matches!(resp, IpcResponse::Ok { .. }));

    // Receive
    let resp = ipc_request(
        &socket_path,
        &IpcRequest::Receive {
            agent: "bob".into(),
            from_filter: None,
            channel_filter: None,
            limit: 10,
            peek: false,
            wait: false,
        },
    )
    .await;
    match resp {
        IpcResponse::MessageList { messages } => {
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].content, "hello via socket");
        }
        _ => panic!("expected MessageList, got {resp:?}"),
    }
}

#[tokio::test]
async fn test_round_trip_context() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("ipc-test.db");
    let socket_path = dir.path().join("ipc.sock");

    let engine = Arc::new(IpcEngine::new(db_path));
    let sock = socket_path.clone();
    tokio::spawn(async move {
        start_ipc_server(engine, sock).await.ok();
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Set context
    let resp = ipc_request(
        &socket_path,
        &IpcRequest::ContextSet {
            key: "plan".into(),
            value: "633".into(),
            set_by: "planner".into(),
        },
    )
    .await;
    assert!(matches!(resp, IpcResponse::Ok { .. }));

    // Get context
    let resp = ipc_request(&socket_path, &IpcRequest::ContextGet { key: "plan".into() }).await;
    match resp {
        IpcResponse::Context { key, value, .. } => {
            assert_eq!(key, "plan");
            assert_eq!(value, "633");
        }
        _ => panic!("expected Context, got {resp:?}"),
    }
}

#[tokio::test]
async fn test_ping_pong() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("ipc-test.db");
    let socket_path = dir.path().join("ipc.sock");

    let engine = Arc::new(IpcEngine::new(db_path));
    let sock = socket_path.clone();
    tokio::spawn(async move {
        start_ipc_server(engine, sock).await.ok();
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let resp = ipc_request(&socket_path, &IpcRequest::Ping).await;
    assert!(matches!(resp, IpcResponse::Pong { .. }));
}

// Plan 635: Intelligence layer integration tests
use claude_core::ipc::{budget, models, router, skills};
use rusqlite::Connection;

fn setup_full_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("
        CREATE TABLE ipc_auth_tokens (id INTEGER PRIMARY KEY, service TEXT, encrypted_token BLOB, nonce BLOB, host TEXT DEFAULT '', updated_at TEXT DEFAULT '', UNIQUE(service, host));
        CREATE TABLE ipc_model_registry (id INTEGER PRIMARY KEY, host TEXT, provider TEXT, model TEXT, size_gb REAL, quantization TEXT, last_seen TEXT, UNIQUE(host, provider, model));
        CREATE TABLE ipc_node_capabilities (host TEXT PRIMARY KEY, provider TEXT, models TEXT, updated_at TEXT);
        CREATE TABLE ipc_subscriptions (name TEXT PRIMARY KEY, provider TEXT, plan TEXT, budget_usd REAL, reset_day INTEGER, models TEXT);
        CREATE TABLE ipc_budget_log (id INTEGER PRIMARY KEY, subscription TEXT, date TEXT, tokens_in INTEGER, tokens_out INTEGER, estimated_cost_usd REAL, model TEXT, task_ref TEXT);
        CREATE TABLE ipc_agent_skills (id INTEGER PRIMARY KEY, agent TEXT, host TEXT, skill TEXT, confidence REAL DEFAULT 0.5, last_used TEXT, UNIQUE(agent, host, skill));
        CREATE TABLE session_state (key TEXT PRIMARY KEY, value TEXT);
    ").unwrap();
    conn
}

#[tokio::test]
async fn test_end_to_end_ipc_flow() {
    let conn = setup_full_db();

    // 1. Add subscription
    let sub = models::Subscription {
        name: "openai-pro".into(),
        provider: "openai".into(),
        plan: "pro".into(),
        budget_usd: 100.0,
        reset_day: 30,
        models: vec!["gpt-4o".into()],
    };
    models::add_subscription(&conn, &sub).unwrap();

    // 2. Register models
    let ms = vec![models::OllamaModel {
        name: "codellama:7b".into(),
        size: 7_000_000_000,
        quantization_level: "Q4_K_M".into(),
    }];
    models::store_models(&conn, "mac-worker-2", "ollama", &ms).unwrap();

    // 3. Log usage
    budget::log_usage(
        &conn,
        &budget::BudgetEntry {
            subscription: "openai-pro".into(),
            date: "2026-03-16".into(),
            tokens_in: 5000,
            tokens_out: 2000,
            estimated_cost_usd: 0.1,
            model: "gpt-4o".into(),
            task_ref: "ipc-test".into(),
        },
    )
    .unwrap();

    // 4. Route a task
    let decision = router::route_task(&conn, "implement a Rust function").unwrap();
    assert!(decision.is_some());
    let d = decision.unwrap();
    assert_eq!(d.provider, "ollama");

    // 5. Register skills and request
    skills::register_skills(&conn, "coder", "mac-worker-2", &[("rust", 0.9)]).unwrap();
    let req_id = skills::create_skill_request(&conn, "rust", "fix borrow checker error").unwrap();
    skills::assign_request(&conn, &req_id, "coder", "mac-worker-2").unwrap();
    skills::complete_skill_request(&conn, &req_id, "added lifetime annotation").unwrap();
    let result = skills::get_request_result(&conn, &req_id).unwrap();
    assert_eq!(result, Some("added lifetime annotation".to_string()));

    // 6. Budget status
    let status = budget::get_budget_status(&conn, "openai-pro")
        .unwrap()
        .unwrap();
    assert_eq!(status.total_spent, 0.1);
    assert!(status.remaining_budget > 99.0);

    // 7. Verify model count
    let all_models = models::get_all_models(&conn).unwrap();
    assert_eq!(all_models.len(), 1);

    // 8. Verify skill pool
    let pool = skills::get_skill_pool(&conn).unwrap();
    assert!(pool.contains_key("rust"));
}
