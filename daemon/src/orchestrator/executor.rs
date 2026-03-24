// Executor — launches claude on a peer via mesh exec + tmux.
// Handles: prompt building, repo discovery, tmux session, completion callback.

use super::actions::{emit, DAEMON_BASE};
use crate::ipc::IpcEngine;
use std::sync::Arc;

type AliResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Delegate a plan to a specific peer: mark in DB, find repo, write prompt, launch claude in tmux.
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

    // 2. Build execution prompt from plan spec
    let prompt = build_plan_prompt(plan_id, &client).await;

    // 3. Find repo path on peer
    let repo_path = find_peer_repo(&client, peer).await?;

    // 4. Sync repo on peer
    let sync_cmd = format!("cd '{repo_path}' && git pull origin main --ff-only 2>&1 | tail -2");
    let _ = exec_on_peer(&client, peer, &sync_cmd).await;

    // 5. Write prompt file on peer
    let prompt_file = format!("/tmp/convergio-plan-{plan_id}.md");
    let escaped = prompt.replace('\'', "'\\''");
    let write_cmd = format!("printf '%s' '{escaped}' > '{prompt_file}'");
    exec_on_peer(&client, peer, &write_cmd).await?;

    // 6. Create tmux session and launch claude with completion callback
    let session = format!("plan-{plan_id}");
    let launch = format!(
        "tmux kill-session -t '{session}' 2>/dev/null; \
         tmux new-session -d -s '{session}' -c '{repo_path}'; \
         tmux send-keys -t '{session}' \
         'claude -p \"$(cat {prompt_file})\" --dangerously-skip-permissions; \
          git push origin main 2>/dev/null; \
          curl -sf -X POST http://localhost:8420/api/ipc/send \
          -H \"Content-Type: application/json\" \
          -d '\\''{{\"sender_name\":\"executor-{peer}\",\"channel\":\"#orchestration\",\
          \"content\":\"{{\\\\\"type\\\\\":\\\\\"plan_done\\\\\",\\\\\"plan_id\\\\\":{plan_id}}}\"}}'\\''; \
          echo PLAN_{plan_id}_DONE' Enter",
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
    let path = body
        .get("stdout")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if path.is_empty() {
        Err(format!("cannot find ConvergioPlatform repo on {peer}").into())
    } else {
        Ok(path)
    }
}

/// Build an execution prompt from plan data.
async fn build_plan_prompt(plan_id: i64, client: &reqwest::Client) -> String {
    let plan_url = format!("{DAEMON_BASE}/api/plan-db/json/{plan_id}");
    let plan_json = match client.get(&plan_url).send().await {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => String::new(),
    };

    format!(
        "You are executing Plan {plan_id} on ConvergioPlatform.\n\
         Read the plan spec and execute all pending tasks in order.\n\
         For each task: implement, test, commit with conventional message.\n\
         When all tasks are done, the completion signal will be sent automatically.\n\n\
         Plan data:\n```json\n{plan_json}\n```\n\n\
         Rules: max 250 lines/file, tests required, cargo check must pass.\n\
         Read CLAUDE.md and CONSTITUTION.md for project conventions."
    )
}
