// Core delegation logic shared by executor (Ali), SSE delegate, and CLI.
// Single source of truth for prompt building, done scripts, and peer ops.

use super::actions::DAEMON_BASE;

type AliResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Build an execution prompt from plan data with per-task cvg workflow.
/// Includes cvg task update lifecycle, cvg plan validate, verify commands.
pub async fn build_plan_prompt(plan_id: i64, client: &reqwest::Client) -> String {
    let url = format!("{DAEMON_BASE}/api/plan-db/json/{plan_id}");
    let plan_json = match client.get(&url).send().await {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => String::new(),
    };
    format!(
        "Execute Plan {plan_id} on ConvergioPlatform.\n\n\
         ## Per-Task Workflow (NON-NEGOTIABLE)\n\
         For EACH task:\n\
         1. `cvg task update <id> in_progress`\n\
         2. Implement + write tests (TDD: RED → GREEN)\n\
         3. Run ALL verify[] commands from the task spec before submitting\n\
         4. `cargo check` + `cargo test` must pass\n\
         5. Commit with conventional commit message\n\
         6. `cvg task update <id> submitted`\n\n\
         ## Per-Wave Workflow\n\
         After completing all tasks in a wave:\n\
         - `cvg plan validate {plan_id}` (Thor validation)\n\n\
         ## Plan Completion\n\
         After all waves are validated:\n\
         - `cvg plan complete {plan_id}`\n\n\
         ## Rules\n\
         - NEVER git push/pull — rsync handles sync between peers\n\
         - NEVER set task status to `done` — only `submitted`. Thor promotes to `done`.\n\
         - Read CLAUDE.md for conventions and coding standards\n\n\
         ## Plan\n```json\n{plan_json}\n```"
    )
}

/// Build the done script that runs on the peer after claude finishes.
/// Posts per-task progress via IPC, rsyncs back, then signals plan completion.
pub fn build_done_script(
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
         COORD='http://{coordinator_dns}:8420'\n\
         # Report per-task progress: scan git log for task commits since delegation\n\
         for task_id in $(git log --oneline --since='1 hour ago' \\\n\
           | grep -oE 'T[0-9]+-[0-9]+' | sort -u); do\n\
           curl -sf -X POST \"$COORD/api/ipc/send\" \\\n\
             -H 'Content-Type: application/json' \\\n\
             -d '{{\"sender_name\":\"executor-{peer}\",\"channel\":\"#orchestration\",\
\"content\":\"{{\\\"type\\\":\\\"task_done\\\",\\\"plan_id\\\":{plan_id},\
\\\"task_id\\\":\\\"'\"$task_id\"'\\\"}}\"}}' 2>/dev/null || true\n\
         done\n\
         # Rsync working files back to coordinator (filesystem sync, NOT git)\n\
         bash scripts/mesh/mesh-rsync.sh '{remote_repo}' '{coordinator_dns}' '{local_repo}'\n\
         # Signal plan completion to Ali via IPC\n\
         curl -sf -X POST \"$COORD/api/ipc/send\" \\\n\
           -H 'Content-Type: application/json' \\\n\
           -d '{{\"sender_name\":\"executor-{peer}\",\"channel\":\"#orchestration\",\
\"content\":\"{{\\\"type\\\":\\\"plan_done\\\",\\\"plan_id\\\":{plan_id}}}\"}}' \\\n\
           2>/dev/null || true\n\
         echo 'PLAN_{plan_id}_DONE'\n"
    )
}

/// Find the local ConvergioPlatform repo path.
pub fn find_local_repo() -> String {
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
pub async fn write_file_on_peer(
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
pub async fn exec_on_peer(
    client: &reqwest::Client,
    peer: &str,
    command: &str,
) -> AliResult {
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
pub async fn find_peer_repo(
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

pub fn base64_encode(data: &[u8]) -> String {
    const C: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let (b0, b1, b2) = (
            chunk[0] as u32,
            chunk.get(1).copied().unwrap_or(0) as u32,
            chunk.get(2).copied().unwrap_or(0) as u32,
        );
        let t = (b0 << 16) | (b1 << 8) | b2;
        out.push(C[((t >> 18) & 0x3F) as usize] as char);
        out.push(C[((t >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 { C[((t >> 6) & 0x3F) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { C[(t & 0x3F) as usize] as char } else { '=' });
    }
    out
}

/// Rsync working files between two machines using mesh-rsync.sh.
pub async fn rsync_files(
    local_path: &str,
    peer_dns: &str,
    remote_path: &str,
) -> AliResult {
    let script = format!("{local_path}/scripts/mesh/mesh-rsync.sh");
    let output = tokio::process::Command::new("bash")
        .args([&script, local_path, peer_dns, remote_path])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rsync to {peer_dns} failed: {stderr}").into());
    }
    tracing::info!("delegation: rsync → {peer_dns} complete");
    Ok(())
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

    #[test]
    fn build_done_script_reports_per_task_progress() {
        let script = build_done_script(742, "peer1", "/remote/repo", "coord", "/local/repo");
        assert!(script.contains("task_done"), "should report per-task progress via IPC");
        assert!(script.contains("task_id"), "should include task_id in IPC payload");
        assert!(script.contains("git log"), "should scan git log for task commits");
    }

    #[test]
    fn build_plan_prompt_includes_cvg_workflow() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let client = reqwest::Client::new();
        let prompt = rt.block_on(build_plan_prompt(999, &client));
        assert!(prompt.contains("cvg task update"), "must include cvg task update");
        assert!(prompt.contains("in_progress"), "must mention in_progress status");
        assert!(prompt.contains("submitted"), "must mention submitted status");
        assert!(prompt.contains("cvg plan validate"), "must include cvg plan validate");
        assert!(prompt.contains("cvg plan complete"), "must include cvg plan complete");
        assert!(prompt.contains("verify[]"), "must mention verify commands");
        assert!(!prompt.contains("status\":\"done\""), "must NOT set done directly");
    }
}
