use super::state::ServerState;
use super::ws_brain::broadcast_brain_task_update;
use super::ws_pty::peer_ssh_alias;
use crate::mesh::delegate::DelegateEngine;
use axum::response::sse::Event;
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

#[path = "sse_delegate_ssh.rs"]
mod ssh;
use ssh::{ssh_connect, ssh_exec, ssh_git_sync};

type Events = Vec<Result<Event, Infallible>>;

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
        map.insert(del_id.clone(), ActiveDelegation { cancelled: Arc::clone(&cancelled) });
    }

    let mut ev = Vec::new();
    push(&mut ev, "delegation_id", &json!({"id": del_id}));
    push(&mut ev, "stage", &stage("connecting", target, "Resolving peer"));

    // DelegateEngine provides peer resolution; SSE flow uses it for validation
    let conf = state.db_path.parent().and_then(|d| d.parent())
        .map(|b| b.join("config/peers.conf")).unwrap_or_default();
    let _engine = DelegateEngine::new(conf);
    let ssh_dest = match peer_ssh_alias(state, target) {
        Some(d) => d,
        None => return do_fail(ev, state, qs, &del_id, "Cannot resolve peer"),
    };
    if cancelled.load(Ordering::Acquire) {
        return cancel_events(ev, state, qs, &del_id);
    }

    push(&mut ev, "stage", &stage("connecting", target, "SSH handshake"));
    let session = match ssh_connect(&ssh_dest) {
        Some(s) => s,
        None => return do_fail(ev, state, qs, &del_id, &format!("SSH to {ssh_dest} failed")),
    };
    if cancelled.load(Ordering::Acquire) {
        return cancel_events(ev, state, qs, &del_id);
    }

    push(&mut ev, "stage", &stage("cloning", target, "git fetch + checkout"));
    if let Err(e) = ssh_git_sync(&session, plan_id) {
        return do_fail(ev, state, qs, &del_id, &e);
    }
    push(&mut ev, "progress", &json!({"percent": 30, "output": "Repository synced"}));
    if cancelled.load(Ordering::Acquire) {
        return cancel_events(ev, state, qs, &del_id);
    }

    push(&mut ev, "stage", &stage("spawning", target, &format!("Launching {cli}")));
    let agent_cmd = build_agent_command(cli, plan_id, qs);
    push(&mut ev, "stage", &stage("running", target, "Agent executing"));

    match ssh_exec(&session, &agent_cmd) {
        Ok((0, output, _)) => {
            emit_progress(&mut ev, &output);
            push(&mut ev, "done", &json!({
                "result": "completed", "plan_id": plan_id,
                "target": target, "peer": target, "delegation_id": del_id
            }));
            update_task_status(state, qs, "done");
            broadcast_ws(state, qs, "done");
        }
        Ok((code, _, stderr)) => {
            return do_fail(ev, state, qs, &del_id, &format!("agent exited {code}: {stderr}"));
        }
        Err(e) => return do_fail(ev, state, qs, &del_id, &e),
    }
    remove_delegation(&del_id);
    ev
}

fn cancel_events(
    mut ev: Events, state: &ServerState, qs: &HashMap<String, String>, del_id: &str,
) -> Events {
    push(&mut ev, "error", &json!({"result": "cancelled"}));
    update_task_status(state, qs, "cancelled");
    broadcast_ws(state, qs, "cancelled");
    remove_delegation(del_id);
    ev
}

fn do_fail(
    mut ev: Events, state: &ServerState, qs: &HashMap<String, String>,
    del_id: &str, msg: &str,
) -> Events {
    push(&mut ev, "error", &json!({"result": msg}));
    update_task_status(state, qs, "failed");
    broadcast_ws(state, qs, "failed");
    remove_delegation(del_id);
    ev
}

fn remove_delegation(del_id: &str) {
    if let Ok(mut map) = active_delegations().lock() { map.remove(del_id); }
}

fn emit_progress(ev: &mut Events, output: &str) {
    let total = output.lines().count().max(1) as u64;
    for (i, line) in output.lines().enumerate() {
        let pct = 30 + ((i as u64 * 70) / total);
        push(ev, "progress", &json!({"percent": pct, "output": line}));
    }
}

fn broadcast_ws(state: &ServerState, qs: &HashMap<String, String>, status: &str) {
    if let Some(tid) = qs.get("task_id").filter(|v| !v.is_empty()) {
        if let Ok(id) = tid.parse::<i64>() {
            broadcast_brain_task_update(state, id, status);
        }
    }
}

fn build_agent_command(cli: &str, plan_id: &str, qs: &HashMap<String, String>) -> String {
    let task_id = qs.get("task_id").cloned().unwrap_or_default();
    let wave_id = qs.get("wave_id").cloned().unwrap_or_default();
    let dir = "~/GitHub/ConvergioPlatform";
    match cli {
        "claude" | "copilot" => {
            let mut cmd = format!(
                "cd {dir} && claude --dangerously-skip-permissions -p 'Execute plan {plan_id}"
            );
            if !task_id.is_empty() { cmd.push_str(&format!(" task {task_id}")); }
            if !wave_id.is_empty() { cmd.push_str(&format!(" wave {wave_id}")); }
            cmd.push('\'');
            cmd
        }
        _ => format!("cd {dir} && {cli} --plan {plan_id}"),
    }
}

fn stage(s: &str, peer: &str, detail: &str) -> serde_json::Value {
    json!({"type": "stage", "stage": s, "peer": peer, "detail": detail})
}

fn push(events: &mut Events, event_type: &str, data: &serde_json::Value) {
    events.push(Ok(Event::default().event(event_type).data(data.to_string())));
}

fn update_task_status(state: &ServerState, qs: &HashMap<String, String>, status: &str) {
    let task_id = match qs.get("task_id").filter(|v| !v.is_empty()) {
        Some(id) => id, None => return,
    };
    let plan_id = match qs.get("plan_id").filter(|v| !v.is_empty()) {
        Some(id) => id, None => return,
    };
    if let Ok(conn) = state.get_conn() {
        if let Err(e) = conn.execute(
            "UPDATE tasks SET status=?1 WHERE plan_id=?2 AND id=?3",
            rusqlite::params![status, plan_id, task_id],
        ) {
            tracing::warn!("delegate task status update failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_cmd_claude_with_params() {
        let mut qs = HashMap::new();
        qs.insert("task_id".into(), "T1-02".into());
        qs.insert("wave_id".into(), "W1".into());
        let cmd = build_agent_command("claude", "671", &qs);
        assert!(cmd.contains("plan 671") && cmd.contains("task T1-02"));
    }

    #[test]
    fn agent_cmd_custom_cli() {
        let cmd = build_agent_command("my-agent", "42", &HashMap::new());
        assert!(cmd.contains("my-agent --plan 42"));
    }

    #[test]
    fn stage_event_fields() {
        let ev = stage("connecting", "worker-1", "SSH handshake");
        assert_eq!(ev["stage"], "connecting");
        assert_eq!(ev["peer"], "worker-1");
    }

    #[test]
    fn cancel_delegation_lifecycle() {
        let del_id = generate_delegation_id("999", "test-peer");
        assert!(!cancel_delegation(&del_id));
        let cancelled = Arc::new(AtomicBool::new(false));
        active_delegations().lock().unwrap().insert(
            del_id.clone(), ActiveDelegation { cancelled: Arc::clone(&cancelled) },
        );
        assert!(cancel_delegation(&del_id));
        assert!(cancelled.load(Ordering::Acquire));
        assert!(list_active_delegations().is_empty());
    }
}
