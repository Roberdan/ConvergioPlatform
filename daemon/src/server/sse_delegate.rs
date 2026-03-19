use super::state::ServerState;
use super::ws_pty::peer_ssh_alias;
use axum::response::sse::Event;
use serde_json::json;
use ssh2::Session;
use std::collections::HashMap;
use std::convert::Infallible;
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

type Events = Vec<Result<Event, Infallible>>;

/// Real plan delegation over SSH: resolve peer, connect, git sync, spawn agent.
pub async fn delegate(
    state: &ServerState,
    qs: &HashMap<String, String>,
    plan_id: &str,
    target: &str,
    cli: &str,
) -> Events {
    let mut ev = Vec::new();
    push(
        &mut ev,
        "stage",
        &stage("connecting", target, "Resolving peer"),
    );
    let ssh_dest = match peer_ssh_alias(state, target) {
        Some(d) => d,
        None => return fail(ev, state, qs, "Cannot resolve peer — check peers.conf"),
    };
    push(
        &mut ev,
        "stage",
        &stage("connecting", target, "SSH handshake"),
    );
    let session = match ssh_connect(&ssh_dest) {
        Some(s) => s,
        None => return fail(ev, state, qs, &format!("SSH to {ssh_dest} failed")),
    };
    push(
        &mut ev,
        "stage",
        &stage("cloning", target, "git fetch + checkout"),
    );
    if let Err(e) = ssh_git_sync(&session, plan_id) {
        return fail(ev, state, qs, &e);
    }
    push(
        &mut ev,
        "progress",
        &json!({"percent": 30, "output": "Repository synced"}),
    );
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
            let total = output.lines().count().max(1) as u64;
            for (i, line) in output.lines().enumerate() {
                let pct = 30 + ((i as u64 * 70) / total);
                push(
                    &mut ev,
                    "progress",
                    &json!({"percent": pct, "output": line}),
                );
            }
            push(
                &mut ev,
                "done",
                &json!({
                    "result": "completed", "plan_id": plan_id,
                    "target": target, "peer": target
                }),
            );
            update_task_status(state, qs, "done");
        }
        Ok((code, _, stderr)) => {
            return fail(ev, state, qs, &format!("agent exited {code}: {stderr}"));
        }
        Err(e) => return fail(ev, state, qs, &e),
    }
    ev
}

fn fail(mut ev: Events, state: &ServerState, qs: &HashMap<String, String>, msg: &str) -> Events {
    push(&mut ev, "error", &json!({"result": msg}));
    update_task_status(state, qs, "failed");
    ev
}

fn ssh_connect(dest: &str) -> Option<Session> {
    ssh_connect_inner(dest).ok()
}

fn ssh_connect_inner(dest: &str) -> Result<Session, String> {
    let (user, host_port) = match dest.split_once('@') {
        Some((u, rest)) => (u.to_string(), rest.to_string()),
        None => (String::new(), dest.to_string()),
    };
    let addr = if host_port.contains(':') {
        host_port
    } else {
        format!("{host_port}:22")
    };
    let tcp = TcpStream::connect_timeout(
        &addr.parse().map_err(|e| format!("bad addr {addr}: {e}"))?,
        Duration::from_secs(10),
    )
    .map_err(|e| format!("TCP to {addr}: {e}"))?;
    let _ = tcp.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = tcp.set_write_timeout(Some(Duration::from_secs(30)));
    let mut session = Session::new().map_err(|e| format!("session: {e}"))?;
    session.set_tcp_stream(tcp);
    session.handshake().map_err(|e| format!("handshake: {e}"))?;
    let auth_user = if user.is_empty() {
        std::env::var("USER").unwrap_or_else(|_| "root".to_string())
    } else {
        user
    };
    session
        .userauth_agent(&auth_user)
        .map_err(|e| format!("auth {auth_user}: {e}"))?;
    if !session.authenticated() {
        return Err(format!("auth failed for {auth_user}"));
    }
    Ok(session)
}

fn ssh_exec(session: &Session, cmd: &str) -> Result<(i32, String, String), String> {
    let mut channel = session.channel_session().map_err(|e| e.to_string())?;
    channel.exec(cmd).map_err(|e| e.to_string())?;
    let mut stdout = String::new();
    let mut stderr = String::new();
    channel
        .read_to_string(&mut stdout)
        .map_err(|e| e.to_string())?;
    channel
        .stderr()
        .read_to_string(&mut stderr)
        .map_err(|e| e.to_string())?;
    let _ = channel.wait_close();
    let code = channel.exit_status().unwrap_or(-1);
    drop(channel);
    Ok((code, stdout, stderr))
}

fn ssh_git_sync(session: &Session, plan_id: &str) -> Result<(), String> {
    let branch = format!("plan-{plan_id}");
    let cmd = format!(
        "cd ~/GitHub/ConvergioPlatform && git fetch origin && \
         (git checkout {branch} 2>/dev/null || \
         git checkout -b {branch} origin/{branch} 2>/dev/null || true) && \
         git pull --ff-only origin {branch} 2>/dev/null || true"
    );
    let (code, _, stderr) = ssh_exec(session, &cmd)?;
    if code != 0 && !stderr.contains("Already on") && !stderr.contains("Already up to date") {
        return Err(format!("git sync exit {code}: {stderr}"));
    }
    Ok(())
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
            if !task_id.is_empty() {
                cmd.push_str(&format!(" task {task_id}"));
            }
            if !wave_id.is_empty() {
                cmd.push_str(&format!(" wave {wave_id}"));
            }
            cmd.push('\'');
            cmd
        }
        _ => format!("cd {dir} && {cli} --plan {plan_id}"),
    }
}

fn stage(stage: &str, peer: &str, detail: &str) -> serde_json::Value {
    json!({"type": "stage", "stage": stage, "peer": peer, "detail": detail})
}

fn push(events: &mut Events, event_type: &str, data: &serde_json::Value) {
    events.push(Ok(Event::default()
        .event(event_type)
        .data(data.to_string())));
}

fn update_task_status(state: &ServerState, qs: &HashMap<String, String>, status: &str) {
    let task_id = match qs.get("task_id").filter(|v| !v.is_empty()) {
        Some(id) => id,
        None => return,
    };
    let plan_id = match qs.get("plan_id").filter(|v| !v.is_empty()) {
        Some(id) => id,
        None => return,
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
        assert!(cmd.contains("plan 671") && cmd.contains("task T1-02") && cmd.contains("wave W1"));
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
        assert_eq!(ev["detail"], "SSH handshake");
    }
}
