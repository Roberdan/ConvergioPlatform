use super::state::ServerState;
use super::ws_pty::peer_ssh_alias;
use crate::mesh::delegate::DelegateEngine;
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

#[path = "sse_delegate_ssh.rs"]
mod ssh;
use ssh::{ssh_connect, ssh_exec, ssh_git_sync};

#[path = "sse_delegate_util.rs"]
pub(super) mod util;
use util::{broadcast_ws, build_agent_command, push, stage, update_task_status, Events};

struct ActiveDelegation {
    cancelled: Arc<AtomicBool>,
}

fn active_delegations() -> &'static Mutex<HashMap<String, ActiveDelegation>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, ActiveDelegation>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Generate a unique delegation ID from plan, target, and timestamp.
pub fn generate_delegation_id(plan_id: &str, target: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("del-{plan_id}-{target}-{ts}")
}

/// Cancel an active delegation. Returns true if found and cancelled.
pub fn cancel_delegation(delegation_id: &str) -> bool {
    if let Ok(mut map) = active_delegations().lock() {
        if let Some(d) = map.remove(delegation_id) {
            d.cancelled.store(true, Ordering::Release);
            return true;
        }
    }
    false
}

/// List active delegation IDs.
pub fn list_active_delegations() -> Vec<String> {
    active_delegations()
        .lock()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

/// Delegate plan execution over SSH with SSE stage events.
/// Broadcasts task status via ws_brain on completion/failure/cancel.
pub async fn delegate(
    state: &ServerState,
    qs: &HashMap<String, String>,
    plan_id: &str,
    target: &str,
    cli: &str,
) -> Events {
    let del_id = generate_delegation_id(plan_id, target);
    let cancelled = Arc::new(AtomicBool::new(false));
    if let Ok(mut map) = active_delegations().lock() {
        map.insert(
            del_id.clone(),
            ActiveDelegation {
                cancelled: Arc::clone(&cancelled),
            },
        );
    }

    let mut ev = Vec::new();
    push(&mut ev, "delegation_id", &json!({"id": del_id}));
    push(
        &mut ev,
        "stage",
        &stage("connecting", target, "Resolving peer"),
    );

    // DelegateEngine provides peer resolution; SSE flow uses it for validation
    let conf = state
        .db_path
        .parent()
        .and_then(|d| d.parent())
        .map(|b| b.join("config/peers.conf"))
        .unwrap_or_default();
    let _engine = DelegateEngine::new(conf);
    let ssh_dest = match peer_ssh_alias(state, target) {
        Some(d) => d,
        None => return do_fail(ev, state, qs, &del_id, "Cannot resolve peer"),
    };
    if cancelled.load(Ordering::Acquire) {
        return cancel_events(ev, state, qs, &del_id);
    }

    push(
        &mut ev,
        "stage",
        &stage("connecting", target, "SSH handshake"),
    );
    let session = match ssh_connect(&ssh_dest) {
        Some(s) => s,
        None => return do_fail(ev, state, qs, &del_id, &format!("SSH to {ssh_dest} failed")),
    };
    if cancelled.load(Ordering::Acquire) {
        return cancel_events(ev, state, qs, &del_id);
    }

    push(
        &mut ev,
        "stage",
        &stage("cloning", target, "git fetch + checkout"),
    );
    if let Err(e) = ssh_git_sync(&session, plan_id) {
        return do_fail(ev, state, qs, &del_id, &e);
    }
    push(
        &mut ev,
        "progress",
        &json!({"percent": 30, "output": "Repository synced"}),
    );
    if cancelled.load(Ordering::Acquire) {
        return cancel_events(ev, state, qs, &del_id);
    }

    push(
        &mut ev,
        "stage",
        &stage("spawning", target, &format!("Launching {cli}")),
    );
    let agent_cmd = build_agent_command(cli, plan_id, qs);
    push(
        &mut ev,
        "stage",
        &stage("running", target, "Agent executing"),
    );

    match ssh_exec(&session, &agent_cmd) {
        Ok((0, output, _)) => {
            emit_progress(&mut ev, &output);
            push(
                &mut ev,
                "done",
                &json!({
                    "result": "completed", "plan_id": plan_id,
                    "target": target, "peer": target, "delegation_id": del_id
                }),
            );
            update_task_status(state, qs, "done");
            broadcast_ws(state, qs, "done");
        }
        Ok((code, _, stderr)) => {
            return do_fail(
                ev,
                state,
                qs,
                &del_id,
                &format!("agent exited {code}: {stderr}"),
            );
        }
        Err(e) => return do_fail(ev, state, qs, &del_id, &e),
    }
    remove_delegation(&del_id);
    ev
}

fn cancel_events(
    mut ev: Events,
    state: &ServerState,
    qs: &HashMap<String, String>,
    del_id: &str,
) -> Events {
    push(&mut ev, "error", &json!({"result": "cancelled"}));
    update_task_status(state, qs, "cancelled");
    broadcast_ws(state, qs, "cancelled");
    remove_delegation(del_id);
    ev
}

fn do_fail(
    mut ev: Events,
    state: &ServerState,
    qs: &HashMap<String, String>,
    del_id: &str,
    msg: &str,
) -> Events {
    push(&mut ev, "error", &json!({"result": msg}));
    update_task_status(state, qs, "failed");
    broadcast_ws(state, qs, "failed");
    remove_delegation(del_id);
    ev
}

fn remove_delegation(del_id: &str) {
    if let Ok(mut map) = active_delegations().lock() {
        map.remove(del_id);
    }
}

fn emit_progress(ev: &mut Events, output: &str) {
    let total = output.lines().count().max(1) as u64;
    for (i, line) in output.lines().enumerate() {
        let pct = 30 + ((i as u64 * 70) / total);
        push(ev, "progress", &json!({"percent": pct, "output": line}));
    }
}

#[cfg(test)]
#[path = "sse_delegate_tests.rs"]
mod tests;
