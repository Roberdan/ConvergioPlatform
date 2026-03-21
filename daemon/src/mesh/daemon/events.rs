// Event publishing and relay helpers

use super::types::{DaemonState, MeshEvent};
use crate::mesh::sync::DeltaChange;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn publish_event(state: &DaemonState, kind: &str, node: &str, payload: Value) {
    let _ = state.tx.send(MeshEvent {
        kind: kind.to_string(),
        node: node.to_string(),
        ts: now_ts(),
        payload,
    });
}

pub fn relay_agent_activity_changes(
    state: &DaemonState,
    node: &str,
    changes: &[DeltaChange],
) {
    let mut grouped: BTreeMap<Vec<u8>, HashMap<String, String>> = BTreeMap::new();
    for change in changes {
        if change.table_name != "agent_activity" {
            continue;
        }
        if let Some(value) = &change.val {
            grouped
                .entry(change.pk.clone())
                .or_default()
                .insert(change.cid.clone(), value.clone());
        }
    }
    for (pk, fields) in grouped {
        let pk_str = String::from_utf8_lossy(&pk).to_string();
        let status = fields.get("status").map_or("running", String::as_str);
        let event_type = match status {
            "running" => "start",
            "completed" | "failed" | "cancelled" => "complete",
            _ => "heartbeat",
        };
        let payload = json!({
            "event_type": event_type,
            "record_key": pk_str,
            "agent_id": fields.get("agent_id").cloned().unwrap_or_default(),
            "status": status,
            "task_db_id": fields.get("task_db_id").cloned(),
            "plan_id": fields.get("plan_id").cloned(),
            "agent_type": fields.get("agent_type").cloned(),
            "model": fields.get("model").cloned(),
            "description": fields.get("description").cloned(),
            "host": fields.get("host").cloned(),
            "region": fields.get("region").cloned(),
            "tokens_in": parse_i64(fields.get("tokens_in")),
            "tokens_out": parse_i64(fields.get("tokens_out")),
            "tokens_total": parse_i64(fields.get("tokens_total")),
        });
        publish_event(state, "agent_heartbeat", node, payload);
    }
}

const IPC_TABLES: &[&str] = &[
    "ipc_agents",
    "ipc_file_locks",
    "ipc_messages",
    "ipc_worktrees",
];

pub fn relay_ipc_changes(state: &DaemonState, node: &str, changes: &[DeltaChange]) {
    for change in changes {
        if !IPC_TABLES.contains(&change.table_name.as_str()) {
            continue;
        }
        let event_kind = match change.table_name.as_str() {
            "ipc_agents" => "ipc_agent_register",
            "ipc_file_locks" => "ipc_lock_change",
            "ipc_messages" => "ipc_message",
            "ipc_worktrees" => "ipc_worktree_change",
            _ => continue,
        };
        let payload = json!({
            "table": change.table_name,
            "cid": change.cid,
            "val": change.val,
        });
        publish_event(state, event_kind, node, payload);
    }
}

fn parse_i64(value: Option<&String>) -> Option<i64> {
    value.and_then(|v| v.parse::<i64>().ok())
}
