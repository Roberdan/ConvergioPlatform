// Executor — launches claude on a peer via mesh exec + tmux.
// Writes a completion script that handles sync-back and IPC callback.

use super::actions::{emit, DAEMON_BASE};
use crate::ipc::IpcEngine;
use std::sync::Arc;

type AliResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Delegate a plan to a specific peer: sync repo, write prompt + completion script, launch claude.
pub async fn delegate_to_peer(engine: &Arc<IpcEngine>, plan_id: i64, peer: &str) -> AliResult {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    tracing::info!("ali: delegating plan {plan_id} to peer {peer}");

    // 1. Mark plan in DB
    let _ = client
        .post(format!("{DAEMON_BASE}/api/mesh/delegate"))
        .json(&serde_json::json!({"plan_id": plan_id, "peer": peer}))
        .send()
        .await;

    // 2. Find repo path on peer
    let repo_path = find_peer_repo(&client, peer).await?;

    // 3. Sync repo: try coordinator remote (SSH), fallback to origin
    let sync = format!(
        "cd '{repo_path}' && \
         (git pull coordinator main --ff-only 2>/dev/null || git pull origin main --ff-only 2>/dev/null) | tail -2"
    );
    let _ = exec_on_peer(&client, peer, &sync).await;

    // 4. Build prompt and write to file
    let prompt = build_plan_prompt(plan_id, &client).await;
    let prompt_file = format!("/tmp/convergio-plan-{plan_id}.md");
    write_file_on_peer(&client, peer, &prompt_file, &prompt).await?;

    // 5. Write completion script (avoids shell escaping hell)
    let done_script = format!("/tmp/convergio-plan-{plan_id}-done.sh");
    let done_content = format!(
        "#!/usr/bin/env bash\n\
         set -euo pipefail\n\
         cd '{repo_path}'\n\
         # Sync back: try coordinator (SSH), fallback to origin\n\
         git push coordinator main 2>/dev/null || git push origin main 2>/dev/null || echo 'WARN: push failed'\n\
         # Signal completion to Ali via IPC\n\
         PAYLOAD='{{\"sender_name\":\"executor-{peer}\",\"channel\":\"#orchestration\",\"content\":\"{{\\\"type\\\":\\\"plan_done\\\",\\\"plan_id\\\":{plan_id}}}\"}}'\n\
         curl -sf -X POST http://localhost:8420/api/ipc/send \\\n\
           -H 'Content-Type: application/json' \\\n\
           -d \"$PAYLOAD\" 2>/dev/null || true\n\
         echo 'PLAN_{plan_id}_DONE'\n"
    );
    write_file_on_peer(&client, peer, &done_script, &done_content).await?;
    exec_on_peer(&client, peer, &format!("chmod +x '{done_script}'")).await?;

    // 6. Launch claude in tmux, run done script on exit
    let session = format!("plan-{plan_id}");
    let launch = format!(
        "tmux kill-session -t '{session}' 2>/dev/null; \
         tmux new-session -d -s '{session}' -c '{repo_path}'; \
         tmux send-keys -t '{session}' \
         'claude -p \"$(cat {prompt_file})\" --dangerously-skip-permissions; bash {done_script}' Enter"
    );
    exec_on_peer(&client, peer, &launch).await?;

    tracing::info!("ali: plan {plan_id} launched on {peer} in tmux:{session}");
    emit(
        engine,
        "plan_delegated",
        &serde_json::json!({"plan_id": plan_id, "peer": peer, "tmux_session": session}),
    )?;
    Ok(())
}

/// Write a file on a peer via mesh exec.
async fn write_file_on_peer(
    client: &reqwest::Client,
    peer: &str,
    path: &str,
    content: &str,
) -> AliResult {
    // Use base64 to avoid all shell escaping issues
    use std::io::Write;
    let mut encoder = Vec::new();
    write!(encoder, "{content}")?;
    let b64 = base64_encode(&encoder);
    let cmd = format!("echo '{b64}' | base64 -d > '{path}'");
    exec_on_peer(client, peer, &cmd).await
}

fn base64_encode(data: &[u8]) -> String {
    use std::fmt::Write;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        let _ = write!(result, "{}", CHARS[((triple >> 18) & 0x3F) as usize] as char);
        let _ = write!(result, "{}", CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            let _ = write!(result, "{}", CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            let _ = write!(result, "{}", CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Execute a command on a peer via mesh exec API.
async fn exec_on_peer(client: &reqwest::Client, peer: &str, command: &str) -> AliResult {
    let resp = client
        .post(format!("{DAEMON_BASE}/api/mesh/exec"))
        .json(&serde_json::json!({
            "peer": peer,
            "command": "bash",
            "args": ["-c", command],
            "timeout_secs": 30,
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
    let plan_url = format!("{DAEMON_BASE}/api/plan-db/json/{plan_id}");
    let plan_json = match client.get(&plan_url).send().await {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => String::new(),
    };

    format!(
        "You are executing Plan {plan_id} on ConvergioPlatform.\n\
         Execute all pending tasks in order. For each task:\n\
         1. Implement the feature/fix\n\
         2. Write tests\n\
         3. Run cargo check (must pass)\n\
         4. Commit with conventional message\n\
         5. IMPORTANT: After each task, mark it done:\n\
            curl -sf -X POST http://localhost:8420/api/plan-db/task/update \\\n\
              -H 'Content-Type: application/json' \\\n\
              -d '{{\"task_id\": \"<TASK_ID>\", \"plan_id\": {plan_id}, \"status\": \"done\", \"notes\": \"<summary>\"}}'  \n\n\
         Plan data:\n```json\n{plan_json}\n```\n\n\
         Rules: max 250 lines/file, tests required, cargo check must pass.\n\
         NEVER use sqlite3 directly — use cvg CLI or daemon API.\n\
         Read CLAUDE.md and CONSTITUTION.md for project conventions."
    )
}
