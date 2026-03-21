// Tests for agents, messaging, and core operations
use super::core::IpcEngine;
use super::super::protocol::{IpcRequest, IpcResponse};

pub(super) fn temp_engine() -> (IpcEngine, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test-ipc.db");
    (IpcEngine::new(db), dir)
}

#[test]
fn test_register_and_who() {
    let (engine, _dir) = temp_engine();
    engine
        .register("planner", "claude", Some(1234), "mac-worker-2", None)
        .unwrap();
    let resp = engine.who().unwrap();
    match resp {
        IpcResponse::AgentList { agents } => {
            assert_eq!(agents.len(), 1);
            assert_eq!(agents[0].name, "planner");
            assert_eq!(agents[0].host, "mac-worker-2");
        }
        _ => panic!("expected AgentList"),
    }
}

#[test]
fn test_register_duplicate() {
    let (engine, _dir) = temp_engine();
    engine
        .register("planner", "claude", Some(1), "mac-worker-2", None)
        .unwrap();
    engine
        .register("planner", "copilot", Some(2), "mac-worker-2", None)
        .unwrap();
    match engine.who().unwrap() {
        IpcResponse::AgentList { agents } => {
            assert_eq!(agents.len(), 1);
            assert_eq!(agents[0].agent_type, "copilot"); // updated
        }
        _ => panic!("expected AgentList"),
    }
}

#[test]
fn test_unregister() {
    let (engine, _dir) = temp_engine();
    engine
        .register("planner", "claude", None, "mac-worker-2", None)
        .unwrap();
    engine.unregister("planner", "mac-worker-2").unwrap();
    match engine.who().unwrap() {
        IpcResponse::AgentList { agents } => assert_eq!(agents.len(), 0),
        _ => panic!("expected AgentList"),
    }
}

#[test]
fn test_send_and_receive() {
    let (engine, _dir) = temp_engine();
    engine
        .register("alice", "claude", None, "mac-worker-2", None)
        .unwrap();
    engine
        .register("bob", "claude", None, "mac-worker-2", None)
        .unwrap();
    engine
        .send_message("alice", "bob", "hello bob", "text", 0)
        .unwrap();

    match engine.receive("bob", None, None, 10, false).unwrap() {
        IpcResponse::MessageList { messages } => {
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].content, "hello bob");
            assert_eq!(messages[0].from_agent, "alice");
        }
        _ => panic!("expected MessageList"),
    }
}

#[test]
fn test_broadcast() {
    let (engine, _dir) = temp_engine();
    engine
        .broadcast("alice", "hello all", "text", None)
        .unwrap();

    match engine.receive("bob", None, None, 10, false).unwrap() {
        IpcResponse::MessageList { messages } => {
            assert_eq!(messages.len(), 1);
            assert!(messages[0].to_agent.is_none());
        }
        _ => panic!("expected MessageList"),
    }
}

#[test]
fn test_receive_peek() {
    let (engine, _dir) = temp_engine();
    engine
        .send_message("alice", "bob", "peek test", "text", 0)
        .unwrap();

    // Peek should NOT mark as read
    engine.receive("bob", None, None, 10, true).unwrap();
    match engine.receive("bob", None, None, 10, false).unwrap() {
        IpcResponse::MessageList { messages } => {
            assert_eq!(messages.len(), 1, "peek should not consume message");
        }
        _ => panic!("expected MessageList"),
    }
}

#[test]
fn test_rate_limit() {
    let (mut engine, _dir) = temp_engine();
    engine.set_rate_limit(3);

    for i in 0..3 {
        let resp = engine
            .send_message("spammer", "bob", &format!("msg{i}"), "text", 0)
            .unwrap();
        assert!(matches!(resp, IpcResponse::Ok { .. }));
    }
    let resp = engine
        .send_message("spammer", "bob", "msg3", "text", 0)
        .unwrap();
    match resp {
        IpcResponse::Error { code, message } => {
            assert_eq!(code, 429);
            assert!(message.contains("exceeded"));
        }
        _ => panic!("expected rate limit error"),
    }
}
