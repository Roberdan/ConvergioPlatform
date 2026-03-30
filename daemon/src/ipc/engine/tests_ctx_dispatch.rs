// Tests for channels, context, db ops, and dispatch
use super::super::protocol::{IpcRequest, IpcResponse};
use super::super::schema::IPC_TABLES;
use super::tests::temp_engine;

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
    engine.register("a", "claude", None, "h", None, None).unwrap();
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

/// db_cleanup must use a parameterized query — not format!() — so the day count
/// is bound as a parameter, not interpolated into the SQL string.
#[test]
fn test_db_cleanup_parameterized_valid_days() {
    let (engine, _dir) = temp_engine();
    // Insert a message and verify cleanup succeeds with valid day values
    engine
        .send_message("alice", "bob", "old message", "text", 0)
        .unwrap();
    // Cleanup with 0 days should delete all messages (created_at < now)
    match engine.db_cleanup(0).unwrap() {
        IpcResponse::Ok { message } => {
            assert!(
                message.contains("cleaned up"),
                "expected cleanup message, got: {message}"
            );
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

/// db_cleanup with a large day value must not delete recent messages.
#[test]
fn test_db_cleanup_preserves_recent_messages() {
    let (engine, _dir) = temp_engine();
    engine
        .send_message("alice", "bob", "recent message", "text", 0)
        .unwrap();
    // 365 days — nothing is older than a year in a fresh DB
    match engine.db_cleanup(365).unwrap() {
        IpcResponse::Ok { message } => {
            assert!(
                message.contains("cleaned up 0"),
                "expected 0 cleaned up, got: {message}"
            );
        }
        other => panic!("expected Ok, got {other:?}"),
    }
    // Message must still be present
    match engine.receive("bob", None, None, 10, false).unwrap() {
        IpcResponse::MessageList { messages } => {
            assert_eq!(messages.len(), 1, "recent message must survive cleanup");
        }
        other => panic!("expected MessageList, got {other:?}"),
    }
}

/// db_reset must clear every table in IPC_TABLES via an explicit match,
/// not a dynamic format!("DELETE FROM {table}") string.
#[test]
fn test_db_reset_clears_all_tables() {
    let (engine, _dir) = temp_engine();
    // Populate multiple tables
    engine.register("a", "claude", None, "host", None, None).unwrap();
    engine.send_message("a", "b", "hello", "text", 0).unwrap();
    engine.channel_create("general", None, "a").unwrap();
    engine.context_set("k", "v", "a").unwrap();

    engine.db_reset().unwrap();

    match engine.db_stats().unwrap() {
        IpcResponse::Stats {
            agents,
            messages,
            channels,
            context_keys,
            ..
        } => {
            assert_eq!(agents, 0, "agents must be cleared after reset");
            assert_eq!(messages, 0, "messages must be cleared after reset");
            assert_eq!(channels, 0, "channels must be cleared after reset");
            assert_eq!(context_keys, 0, "context must be cleared after reset");
        }
        other => panic!("expected Stats, got {other:?}"),
    }
}

/// db_reset must handle each table in IPC_TABLES — verify the count of tables matches.
#[test]
fn test_ipc_tables_constant_coverage() {
    // IPC_TABLES must list exactly the 6 known tables; if a new table is added
    // the explicit match in db_reset must be updated too.
    assert_eq!(
        IPC_TABLES.len(),
        6,
        "IPC_TABLES has changed — update db_reset explicit match accordingly"
    );
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
