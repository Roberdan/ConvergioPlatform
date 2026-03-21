// Relay and resilience tests for daemon event broadcasting.

use crate::mesh::sync::DeltaChange;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[test]
fn relays_agent_start_as_agent_heartbeat_event() {
    let (tx, mut rx) = broadcast::channel(16);
    let state = super::super::DaemonState {
        node_id: "local-node".to_string(),
        tx,
        heartbeats: Arc::new(RwLock::new(HashMap::new())),
    };
    let changes = vec![
        DeltaChange {
            table_name: "agent_activity".to_string(),
            pk: b"id=1".to_vec(),
            cid: "agent_id".to_string(),
            val: Some("agent-123".to_string()),
            col_version: 1,
            db_version: 11,
            site_id: b"peer-a".to_vec(),
            cl: 1,
            seq: 1,
        },
        DeltaChange {
            table_name: "agent_activity".to_string(),
            pk: b"id=1".to_vec(),
            cid: "status".to_string(),
            val: Some("running".to_string()),
            col_version: 1,
            db_version: 12,
            site_id: b"peer-a".to_vec(),
            cl: 1,
            seq: 2,
        },
        DeltaChange {
            table_name: "agent_activity".to_string(),
            pk: b"id=1".to_vec(),
            cid: "model".to_string(),
            val: Some("gpt-5.3-codex".to_string()),
            col_version: 1,
            db_version: 12,
            site_id: b"peer-a".to_vec(),
            cl: 1,
            seq: 3,
        },
    ];
    super::super::relay_agent_activity_changes(&state, "peer-a:9420", &changes);
    let event = rx.try_recv().expect("event");
    assert_eq!(event.kind, "agent_heartbeat");
    assert_eq!(event.node, "peer-a:9420");
    assert_eq!(event.payload["event_type"], "start");
    assert_eq!(event.payload["agent_id"], "agent-123");
    assert_eq!(event.payload["model"], "gpt-5.3-codex");
}

#[test]
fn relays_agent_complete_tokens_and_task_transition() {
    let (tx, mut rx) = broadcast::channel(16);
    let state = super::super::DaemonState {
        node_id: "local-node".to_string(),
        tx,
        heartbeats: Arc::new(RwLock::new(HashMap::new())),
    };
    let changes = vec![
        DeltaChange {
            table_name: "agent_activity".to_string(),
            pk: b"id=9".to_vec(),
            cid: "agent_id".to_string(),
            val: Some("agent-9".to_string()),
            col_version: 1,
            db_version: 21,
            site_id: b"peer-b".to_vec(),
            cl: 1,
            seq: 1,
        },
        DeltaChange {
            table_name: "agent_activity".to_string(),
            pk: b"id=9".to_vec(),
            cid: "status".to_string(),
            val: Some("completed".to_string()),
            col_version: 1,
            db_version: 21,
            site_id: b"peer-b".to_vec(),
            cl: 1,
            seq: 2,
        },
        DeltaChange {
            table_name: "agent_activity".to_string(),
            pk: b"id=9".to_vec(),
            cid: "task_db_id".to_string(),
            val: Some("6810".to_string()),
            col_version: 1,
            db_version: 21,
            site_id: b"peer-b".to_vec(),
            cl: 1,
            seq: 3,
        },
        DeltaChange {
            table_name: "agent_activity".to_string(),
            pk: b"id=9".to_vec(),
            cid: "tokens_total".to_string(),
            val: Some("1200".to_string()),
            col_version: 1,
            db_version: 22,
            site_id: b"peer-b".to_vec(),
            cl: 1,
            seq: 4,
        },
    ];
    super::super::relay_agent_activity_changes(&state, "peer-b:9420", &changes);
    let event = rx.try_recv().expect("event");
    assert_eq!(event.kind, "agent_heartbeat");
    assert_eq!(event.payload["event_type"], "complete");
    assert_eq!(event.payload["task_db_id"], "6810");
    assert_eq!(event.payload["tokens_total"], 1200);
}

#[test]
fn daemon_state_broadcast_full_channel_does_not_panic() {
    let (tx, _rx) = broadcast::channel(1);
    let state = super::super::DaemonState {
        node_id: "test".to_string(),
        tx,
        heartbeats: Arc::new(RwLock::new(HashMap::new())),
    };
    // Fill channel beyond capacity — should not panic
    for i in 0..10 {
        let _ = state.tx.send(super::super::MeshEvent {
            kind: "test".into(),
            node: format!("n{i}"),
            ts: 0,
            payload: serde_json::json!({}),
        });
    }
}

#[test]
fn empty_delta_changes_relay_no_events() {
    let (tx, mut rx) = broadcast::channel(16);
    let state = super::super::DaemonState {
        node_id: "local".to_string(),
        tx,
        heartbeats: Arc::new(RwLock::new(HashMap::new())),
    };
    super::super::relay_agent_activity_changes(&state, "peer-a:9420", &[]);
    assert!(rx.try_recv().is_err(), "no events from empty changes");
}

#[test]
fn non_agent_table_changes_ignored_by_relay() {
    let (tx, mut rx) = broadcast::channel(16);
    let state = super::super::DaemonState {
        node_id: "local".to_string(),
        tx,
        heartbeats: Arc::new(RwLock::new(HashMap::new())),
    };
    let changes = vec![DeltaChange {
        table_name: "plans".to_string(),
        pk: b"id=1".to_vec(),
        cid: "name".to_string(),
        val: Some("test-plan".to_string()),
        col_version: 1,
        db_version: 1,
        site_id: b"peer".to_vec(),
        cl: 1,
        seq: 1,
    }];
    super::super::relay_agent_activity_changes(&state, "peer-a:9420", &changes);
    assert!(
        rx.try_recv().is_err(),
        "non-agent_activity changes should not emit events"
    );
}
