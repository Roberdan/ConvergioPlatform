// Executor — launches claude on a peer via rsync + tmux.
// Sync model: rsync for working files, CRDT for DB state, git push only for finished commits.
// NEVER use git push/pull between peers — rsync preserves uncommitted work.
// Delegates prompt/script building to delegation_core (shared with SSE delegate + CLI).

use super::actions::{emit, DAEMON_BASE};
use super::delegation_core;
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

    let local_repo = delegation_core::find_local_repo();
    let remote_repo = delegation_core::find_peer_repo(&client, peer).await?;
    let coordinator_dns = IpcEngine::hostname();

    // Rsync coordinator → peer (working files, NOT git)
    delegation_core::rsync_files(&local_repo, peer, &remote_repo).await?;

    // Write prompt file on peer (uses shared prompt builder with cvg workflow)
    let prompt = delegation_core::build_plan_prompt(plan_id, &client).await;
    let prompt_file = format!("/tmp/convergio-plan-{plan_id}.md");
    delegation_core::write_file_on_peer(&client, peer, &prompt_file, &prompt).await?;

    // Write completion script: per-task IPC + rsync back + plan_done signal
    let done_script_path = format!("/tmp/convergio-plan-{plan_id}-done.sh");
    let done_content =
        delegation_core::build_done_script(plan_id, peer, &remote_repo, &coordinator_dns, &local_repo);
    delegation_core::write_file_on_peer(&client, peer, &done_script_path, &done_content).await?;
    delegation_core::exec_on_peer(&client, peer, &format!("chmod +x '{done_script_path}'")).await?;

    // Launch in unified "Convergio" tmux session — one session per node, one window per plan.
    let session = "Convergio";
    let window = format!("plan-{plan_id}");
    let launch = format!(
        "tmux has-session -t '{session}' 2>/dev/null || \
         tmux new-session -d -s '{session}' -n 'kernel' -c '{remote_repo}'; \
         tmux new-window -t '{session}' -n '{window}' -c '{remote_repo}'; \
         tmux send-keys -t '{session}:{window}' \
         'claude -p \"$(cat {prompt_file})\" --dangerously-skip-permissions; \
         bash {done_script_path}' Enter"
    );
    delegation_core::exec_on_peer(&client, peer, &launch).await?;

    tracing::info!("ali: plan {plan_id} launched on {peer} in tmux:Convergio:{window}");
    emit(engine, "plan_delegated", &serde_json::json!({
        "plan_id": plan_id, "peer": peer, "tmux_session": session, "tmux_window": window,
    }))?;
    Ok(())
}
