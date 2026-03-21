// Tests for channels, context, db ops, and dispatch
use super::tests::temp_engine;
use super::super::protocol::{IpcRequest, IpcResponse};

#[test]
fn test_channel_create_and_list() {
    let (engine, _dir) = temp_engine();
    engine
        .channel_create("general", Some("general chat"), "alice")
        .unwrap();
    engine.channel_create("ops", None, "bob").unwrap();

    match engine.channel_list().unwrap() {
        IpcResponse::ChannelList { channels } => {
            assert_eq!(channels.len(), 2);
        }
        _ => panic!("expected ChannelList"),
    }
}

#[test]
fn test_context_set_get_lww() {
    let (engine, _dir) = temp_engine();
    engine.context_set("plan_id", "633", "planner").unwrap();
    engine.context_set("plan_id", "634", "executor").unwrap();

    match engine.context_get("plan_id").unwrap() {
        IpcResponse::Context {
            value,
            version,
            set_by,
            ..
        } => {
            assert_eq!(value, "634");
            assert_eq!(version, 2);
            assert_eq!(set_by, "executor");
        }
        _ => panic!("expected Context"),
    }
}

#[test]
fn test_context_delete() {
    let (engine, _dir) = temp_engine();
    engine.context_set("key1", "val1", "agent").unwrap();
    engine.context_delete("key1").unwrap();

    match engine.context_get("key1").unwrap() {
        IpcResponse::Error { code, .. } => assert_eq!(code, 404),
        _ => panic!("expected Error"),
    }
}

#[test]
fn test_history() {
    let (engine, _dir) = temp_engine();
    engine
        .send_message("alice", "bob", "msg1", "text", 0)
        .unwrap();
    engine.broadcast("alice", "msg2", "text", None).unwrap();

    match engine.history(Some("alice"), None, 50, None).unwrap() {
        IpcResponse::MessageList { messages } => {
            assert_eq!(messages.len(), 2);
        }
        _ => panic!("expected MessageList"),
    }
}

#[test]
fn test_db_stats_and_reset() {
    let (engine, _dir) = temp_engine();
    engine.register("a", "claude", None, "h", None).unwrap();
    engine.send_message("a", "b", "x", "text", 0).unwrap();

    match engine.db_stats().unwrap() {
        IpcResponse::Stats {
            agents, messages, ..
        } => {
            assert_eq!(agents, 1);
            assert_eq!(messages, 1);
        }
        _ => panic!("expected Stats"),
    }

    engine.db_reset().unwrap();
    match engine.db_stats().unwrap() {
        IpcResponse::Stats {
            agents, messages, ..
        } => {
            assert_eq!(agents, 0);
            assert_eq!(messages, 0);
        }
        _ => panic!("expected Stats"),
    }
}

#[test]
fn test_context_set_get_delete() {
    let (engine, _dir) = temp_engine();
    engine.context_set("k1", "v1", "agent").unwrap();
    match engine.context_get("k1").unwrap() {
        IpcResponse::Context { value, .. } => assert_eq!(value, "v1"),
        _ => panic!("expected Context"),
    }
    engine.context_delete("k1").unwrap();
    match engine.context_get("k1").unwrap() {
        IpcResponse::Error { code, .. } => assert_eq!(code, 404),
        _ => panic!("expected Error after delete"),
    }
}

#[tokio::test]
async fn test_dispatch_routing() {
    let (engine, _dir) = temp_engine();

    let resp = engine.dispatch(IpcRequest::Ping).await.unwrap();
    match resp {
        IpcResponse::Pong { .. } => {}
        _ => panic!("expected Pong"),
    }

    let resp = engine
        .dispatch(IpcRequest::Register {
            name: "test".into(),
            agent_type: "claude".into(),
            pid: None,
            host: "local".into(),
            metadata: None,
        })
        .await
        .unwrap();
    match resp {
        IpcResponse::Ok { .. } => {}
        _ => panic!("expected Ok"),
    }

    let resp = engine.dispatch(IpcRequest::Who).await.unwrap();
    match resp {
        IpcResponse::AgentList { agents } => assert_eq!(agents.len(), 1),
        _ => panic!("expected AgentList"),
    }
}
