use convergiomesh_core::{
    coordinator::{load_migration_state, migrate_coordinator, rollback},
    peers::PeersRegistry,
};

use super::helpers::{default_peers_path, json_err, json_ok};

pub async fn handle_coordinator_migrate(to: String) {
    let peers_path = default_peers_path();
    if !peers_path.exists() {
        json_err("peers.conf not found — cannot migrate coordinator");
    }
    let mut registry = PeersRegistry::load(&peers_path)
        .unwrap_or_else(|e| json_err(&format!("load peers failed: {e}")));

    let current_from = registry.get_coordinator()
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| json_err("no current coordinator found in peers.conf"));

    let state = migrate_coordinator(&mut registry, &current_from, &to).await
        .unwrap_or_else(|e| json_err(&format!("migrate_coordinator failed: {e}")));

    registry.save(&peers_path)
        .unwrap_or_else(|e| json_err(&format!("save peers failed: {e}")));

    json_ok(serde_json::json!({
        "old_coordinator": state.old_coordinator,
        "new_coordinator": state.new_coordinator,
        "completed": state.completed,
        "started_at": state.started_at
    }));
}

pub async fn handle_coordinator_rollback(snapshot: String) {
    let data = std::fs::read(&snapshot)
        .unwrap_or_else(|e| json_err(&format!("read snapshot: {e}")));
    let state: convergiomesh_core::coordinator::MigrationState =
        serde_json::from_slice(&data)
        .unwrap_or_else(|e| json_err(&format!("parse snapshot: {e}")));

    rollback(&state).await
        .unwrap_or_else(|e| json_err(&format!("rollback failed: {e}")));

    json_ok(serde_json::json!({
        "rolled_back_to": state.old_coordinator,
        "from": state.new_coordinator
    }));
}

pub fn handle_coordinator_status() {
    match load_migration_state() {
        Ok(state) => {
            json_ok(serde_json::json!({
                "old_coordinator": state.old_coordinator,
                "new_coordinator": state.new_coordinator,
                "completed": state.completed,
                "started_at": state.started_at,
                "snapshots": state.snapshots.len()
            }));
        }
        Err(_) => {
            // No migration state file — that's normal
            let peers_path = default_peers_path();
            let coordinator = if peers_path.exists() {
                PeersRegistry::load(&peers_path).ok()
                    .and_then(|r| r.get_coordinator().map(|(n, _)| n.to_string()))
            } else {
                None
            };
            json_ok(serde_json::json!({
                "migration_in_progress": false,
                "current_coordinator": coordinator
            }));
        }
    }
}
