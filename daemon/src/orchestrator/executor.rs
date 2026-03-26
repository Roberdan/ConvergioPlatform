// Executor — launches claude on a peer via rsync + tmux.
// Sync model: rsync for working files, CRDT for DB state, git push only for finished commits.
// NEVER use git push/pull between peers — rsync preserves uncommitted work.

use super::actions::{emit, DAEMON_BASE};
use crate::ipc::IpcEngine;
use std::sync::Arc;

type AliResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Delegate a plan to a specific peer:
/// 1. rsync coordinator → peer (working files)
/// 2. Write prompt + completion script on peer
/// 3. Launch claude in tmux
/// 4. On completion: rsync peer → coordinator (done script handles this)
pub async fn delegate_to_peer(engine: &Arc<IpcEngine>, plan_id: i64, peer: &str) -> AliResult {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_default();

    tracing::info!("ali: delegating plan {plan_id} to peer {peer}");

    // Mark plan in DB via API
    let _ = client
        .post(format!("{DAEMON_BASE}/api/mesh/delegate"))
        .json(&serde_json::json!({"plan_id": plan_id, "peer": peer}))
        .send()
        .await;

    let local_repo = find_local_repo();
    let remote_repo = find_peer_repo(&client, peer).await?;
    let coordinator_dns = IpcEngine::hostname();

    // Rsync coordinator → peer (working files, NOT git)
    rsync_files(&local_repo, peer, &remote_repo).await?;

    // Write prompt file on peer
    let prompt = build_plan_prompt(plan_id, &client).await;
    let prompt_file = format!("/tmp/convergio-plan-{plan_id}.md");
    write_file_on_peer(&client, peer, &prompt_file, &prompt).await?;

    // Write completion script: rsync back + IPC callback
    let done_script = format!("/tmp/convergio-plan-{plan_id}-done.sh");
    let done_content = build_done_script(plan_id, peer, &remote_repo, &coordinator_dns, &local_repo);
    write_file_on_peer(&client, peer, &done_script, &done_content).await?;
    exec_on_peer(&client, peer, &format!("chmod +x '{done_script}'")).await?;

    // Launch claude in tmux
    let session = format!("plan-{plan_id}");
    let launch = format!(
        "tmux kill-session -t '{session}' 2>/dev/null; \
         tmux new-session -d -s '{session}' -c '{remote_repo}'; \
         tmux send-keys -t '{session}' \
         'claude -p \"$(cat {prompt_file})\" --dangerously-skip-permissions; \
         bash {done_script}' Enter"
    );
    exec_on_peer(&client, peer, &launch).await?;

    tracing::info!("ali: plan {plan_id} launched on {peer} in tmux:{session}");
    emit(engine, "plan_delegated", &serde_json::json!({
        "plan_id": plan_id, "peer": peer, "tmux_session": session,
    }))?;
    Ok(())
}

/// Rsync working files between two machines using mesh-rsync.sh.
/// Runs locally — rsync handles SSH transport to the peer.
async fn rsync_files(local_path: &str, peer_dns: &str, remote_path: &str) -> AliResult {
    let script = format!("{local_path}/scripts/mesh/mesh-rsync.sh");
    let output = tokio::process::Command::new("bash")
        .args([&script, local_path, peer_dns, remote_path])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rsync to {peer_dns} failed: {stderr}").into());
    }
    tracing::info!("ali: rsync → {peer_dns} complete");
    Ok(())
}

/// Build the done script that runs on the peer after claude finishes.
/// Rsyncs working files back to coordinator, then signals completion via IPC.
fn build_done_script(
    plan_id: i64,
    peer: &str,
    remote_repo: &str,
    coordinator_dns: &str,
    local_repo: &str,
) -> String {
    format!(
        "#!/usr/bin/env bash\n\
         set -euo pipefail\n\
         cd '{remote_repo}'\n\
         # Rsync working files back to coordinator (filesystem sync, NOT git)\n\
         bash scripts/mesh/mesh-rsync.sh '{remote_repo}' '{coordinator_dns}' '{local_repo}'\n\
         # Signal completion to Ali via IPC on coordinator daemon\n\
         COORD='http://{coordinator_dns}:8420'\n\
         PAYLOAD='{{\"sender_name\":\"executor-{peer}\",\"channel\":\"#orchestration\",\
\"content\":\"{{\\\"type\\\":\\\"plan_done\\\",\\\"plan_id\\\":{plan_id}}}\"}}'\n\
         curl -sf -X POST \"$COORD/api/ipc/send\" \\\n\
           -H 'Content-Type: application/json' -d \"$PAYLOAD\" 2>/dev/null || true\n\
         echo 'PLAN_{plan_id}_DONE'\n"
    )
}

/// Find the local ConvergioPlatform repo path.
fn find_local_repo() -> String {
    if let Ok(repo) = std::env::var("CONVERGIO_REPO") {
        if std::path::Path::new(&repo).join("CLAUDE.md").exists() {
            return repo;
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let default = format!("{home}/GitHub/ConvergioPlatform");
    if std::path::Path::new(&default).join("CLAUDE.md").exists() {
        return default;
    }
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string())
}

/// Write a file on a peer via mesh exec using base64 transfer.
async fn write_file_on_peer(
    client: &reqwest::Client,
    peer: &str,
    path: &str,
    content: &str,
) -> AliResult {
    let b64 = base64_encode(content.as_bytes());
    let cmd = format!("echo '{b64}' | base64 -d > '{path}'");
    exec_on_peer(client, peer, &cmd).await
}

/// Execute a command on a peer via mesh exec API.
async fn exec_on_peer(client: &reqwest::Client, peer: &str, command: &str) -> AliResult {
    let resp = client
        .post(format!("{DAEMON_BASE}/api/mesh/exec"))
        .json(&serde_json::json!({
            "peer": peer,
            "command": "bash",
            "args": ["-c", command],
            "timeout_secs": 120,
        }))
        .send()
        .await?;

    let body: serde_json::Value = resp.json().await?;
    if body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        Ok(())
    } else {
        let stderr = body.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
        Err(format!("exec on {peer}: {stderr}").into())
    }
}

/// Find the ConvergioPlatform repo path on a peer.
async fn find_peer_repo(
    client: &reqwest::Client,
    peer: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let resp = client
        .post(format!("{DAEMON_BASE}/api/mesh/exec"))
        .json(&serde_json::json!({
            "peer": peer,
            "command": "bash",
            "args": ["-c", "find /Users -maxdepth 4 -name ConvergioPlatform -type d 2>/dev/null | grep GitHub | head -1"],
            "timeout_secs": 10,
        }))
        .send()
        .await?;

    let body: serde_json::Value = resp.json().await?;
    let path = body.get("stdout").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if path.is_empty() {
        Err(format!("cannot find ConvergioPlatform repo on {peer}").into())
    } else {
        Ok(path)
    }
}

/// Build an execution prompt from plan data with per-task IPC callbacks.
async fn build_plan_prompt(plan_id: i64, client: &reqwest::Client) -> String {
    let url = format!("{DAEMON_BASE}/api/plan-db/json/{plan_id}");
    let plan_json = match client.get(&url).send().await {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => String::new(),
    };
    format!(
        "Execute Plan {plan_id} on ConvergioPlatform. Per task: implement, test, cargo check, \
         commit, then mark done via API. NEVER git push/pull — rsync handles sync.\n\
         Mark done: curl -sf -X POST http://localhost:8420/api/plan-db/task/update \
         -H 'Content-Type: application/json' \
         -d '{{\"task_id\":\"<ID>\",\"plan_id\":{plan_id},\"status\":\"done\"}}'\n\
         Plan:\n```json\n{plan_json}\n```\nRead CLAUDE.md for conventions."
    )
}

fn base64_encode(data: &[u8]) -> String {
    const C: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let (b0, b1, b2) = (chunk[0] as u32, chunk.get(1).copied().unwrap_or(0) as u32, chunk.get(2).copied().unwrap_or(0) as u32);
        let t = (b0 << 16) | (b1 << 8) | b2;
        out.push(C[((t >> 18) & 0x3F) as usize] as char);
        out.push(C[((t >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 { C[((t >> 6) & 0x3F) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { C[(t & 0x3F) as usize] as char } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encodes_correctly() {
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"ab"), "YWI=");
    }

    #[test]
    fn find_local_repo_returns_valid_path() {
        let repo = find_local_repo();
        assert!(!repo.is_empty(), "repo path should not be empty");
    }

    #[test]
    fn build_done_script_contains_rsync_not_git() {
        let script = build_done_script(719, "peer1", "/remote/repo", "coordinator", "/local/repo");
        assert!(script.contains("mesh-rsync.sh"), "should use rsync");
        assert!(!script.contains("git push"), "must NOT use git push");
        assert!(!script.contains("git pull"), "must NOT use git pull");
        assert!(script.contains("coordinator:8420"), "should callback to coordinator");
    }

    #[test]
    fn build_done_script_is_valid_bash() {
        let script = build_done_script(100, "mac-m1", "/Users/test/repo", "mac-m5", "/Users/m5/repo");
        assert!(script.starts_with("#!/usr/bin/env bash"));
        assert!(script.contains("set -euo pipefail"));
    }
}
