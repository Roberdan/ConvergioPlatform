// Individual steps for the full automated delegation pipeline.
// Each step uses tokio::process::Command with explicit timeouts.
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

type PipelineResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Peer fields loaded from peers.conf for one named peer.
#[derive(Debug)]
pub struct PeerConfig {
    pub name: String,
    pub ssh_alias: String,
    pub tailscale_ip: String,
    pub user: String,
}

/// Load peer config from peers.conf by peer name.
pub fn load_peer_config(
    peer_name: &str,
    peers: &HashMap<String, HashMap<String, String>>,
) -> PipelineResult<PeerConfig> {
    let fields = peers.get(peer_name).ok_or_else(|| {
        format!("peer '{peer_name}' not found in peers.conf")
    })?;
    let ssh_alias = fields.get("ssh_alias").cloned().unwrap_or_else(|| peer_name.to_string());
    let tailscale_ip = fields.get("tailscale_ip").cloned().unwrap_or_default();
    let user = fields.get("user").cloned().unwrap_or_else(|| "roberdan".to_string());
    Ok(PeerConfig { name: peer_name.to_string(), ssh_alias, tailscale_ip, user })
}

/// Step 2: verify peer is reachable via tailscale ping or SSH fallback.
pub async fn verify_peer_reachable(cfg: &PeerConfig) -> PipelineResult<()> {
    if !cfg.tailscale_ip.is_empty() {
        let result = timeout(
            Duration::from_secs(10),
            tokio::process::Command::new("tailscale")
                .args(["ping", "-c", "1", &cfg.tailscale_ip])
                .output(),
        )
        .await;
        if let Ok(Ok(out)) = result {
            if out.status.success() {
                tracing::info!("delegation: peer {} reachable via tailscale", cfg.name);
                return Ok(());
            }
        }
    }
    // Fallback: SSH connectivity check
    let result = timeout(
        Duration::from_secs(15),
        tokio::process::Command::new("ssh")
            .args(["-o", "ConnectTimeout=10", "-o", "BatchMode=yes", &cfg.ssh_alias, "true"])
            .output(),
    )
    .await;
    match result {
        Ok(Ok(out)) if out.status.success() => {
            tracing::info!("delegation: peer {} reachable via SSH", cfg.name);
            Ok(())
        }
        Ok(Ok(out)) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err(format!("peer {} unreachable (SSH): {stderr}", cfg.name).into())
        }
        Ok(Err(e)) => Err(format!("SSH connect error: {e}").into()),
        Err(_) => Err(format!("peer {} SSH check timed out", cfg.name).into()),
    }
}

/// Step 3: rsync the daemon binary to the peer.
pub async fn rsync_binary(cfg: &PeerConfig, local_repo: &str) -> PipelineResult<()> {
    let src = format!("{local_repo}/daemon/target/release/convergio-platform-daemon");
    let dst = format!(
        "{}@{}:~/GitHub/ConvergioPlatform/daemon/target/release/",
        cfg.user, cfg.ssh_alias
    );
    let out = timeout(
        Duration::from_secs(120),
        tokio::process::Command::new("rsync")
            .args(["-az", &src, &dst])
            .output(),
    )
    .await
    .map_err(|_| "rsync binary timed out")??;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("rsync binary failed: {stderr}").into());
    }
    tracing::info!("delegation: binary rsynced to {}", cfg.ssh_alias);
    Ok(())
}

/// Step 4: rsync repo source (excluding target/, .git/objects, node_modules/, data/).
pub async fn rsync_repo_source(cfg: &PeerConfig, local_repo: &str) -> PipelineResult<()> {
    let dst = format!(
        "{}@{}:~/GitHub/ConvergioPlatform/",
        cfg.user, cfg.ssh_alias
    );
    let out = timeout(
        Duration::from_secs(180),
        tokio::process::Command::new("rsync")
            .args([
                "-az", "--delete",
                "--exclude=target/",
                "--exclude=.git/objects",
                "--exclude=node_modules/",
                "--exclude=data/",
                &format!("{local_repo}/"),
                &dst,
            ])
            .output(),
    )
    .await
    .map_err(|_| "rsync repo timed out")??;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("rsync repo failed: {stderr}").into());
    }
    tracing::info!("delegation: repo rsynced to {}", cfg.ssh_alias);
    Ok(())
}

/// Step 5: create a detached worktree on the peer (NEVER -b).
pub async fn create_peer_worktree(cfg: &PeerConfig, plan_id: i64) -> PipelineResult<()> {
    let cmd = format!(
        "cd ~/GitHub/ConvergioPlatform && git worktree add --detach /private/tmp/wt-plan-{plan_id} HEAD"
    );
    let out = timeout(
        Duration::from_secs(30),
        tokio::process::Command::new("ssh")
            .args([&cfg.ssh_alias, &cmd])
            .output(),
    )
    .await
    .map_err(|_| "worktree creation timed out")??;
    // Exit 128 usually means worktree already exists — treat as OK
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        if !stderr.contains("already exists") && !stderr.contains("is already checked out") {
            return Err(format!("worktree create failed: {stderr}").into());
        }
    }
    tracing::info!("delegation: worktree /private/tmp/wt-plan-{plan_id} ready on {}", cfg.ssh_alias);
    Ok(())
}

/// Step 7: check peer daemon version via GET /health and restart if mismatched.
pub async fn ensure_peer_daemon(cfg: &PeerConfig, local_version: &str) -> PipelineResult<()> {
    let peer_ip = if cfg.tailscale_ip.is_empty() { cfg.ssh_alias.clone() } else { cfg.tailscale_ip.clone() };
    let health_url = format!("http://{peer_ip}:8420/api/health");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let needs_restart = match client.get(&health_url).send().await {
        Ok(r) => {
            let body: serde_json::Value = r.json().await.unwrap_or_default();
            let remote_ver = body.get("version").and_then(|v| v.as_str()).unwrap_or("");
            !remote_ver.is_empty() && remote_ver != local_version
        }
        Err(_) => true, // Not running — need to start
    };
    if needs_restart {
        tracing::info!("delegation: restarting daemon on {} (version mismatch)", cfg.ssh_alias);
        let restart_cmd = "pkill -f convergio-platform-daemon 2>/dev/null || true; \
            sleep 2; \
            nohup ~/GitHub/ConvergioPlatform/daemon/target/release/convergio-platform-daemon \
            > /tmp/daemon.log 2>&1 &";
        let out = timeout(
            Duration::from_secs(20),
            tokio::process::Command::new("ssh")
                .args([&cfg.ssh_alias, restart_cmd])
                .output(),
        )
        .await
        .map_err(|_| "daemon restart timed out")??;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!("daemon restart failed: {stderr}").into());
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    Ok(())
}

/// Step 8: detect CLI on peer, write prompt, launch execution in tmux.
pub async fn launch_peer_execution(
    cfg: &PeerConfig,
    plan_id: i64,
    prompt: &str,
) -> PipelineResult<()> {
    // Detect CLI: prefer copilot, fall back to claude
    let detect_cmd = "which copilot 2>/dev/null || which claude 2>/dev/null || echo 'not-found'";
    let out = timeout(
        Duration::from_secs(10),
        tokio::process::Command::new("ssh")
            .args([&cfg.ssh_alias, detect_cmd])
            .output(),
    )
    .await
    .map_err(|_| "CLI detection timed out")??;
    let cli_path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if cli_path == "not-found" || cli_path.is_empty() {
        return Err(format!("no CLI (copilot/claude) found on {}", cfg.ssh_alias).into());
    }
    let cli_binary = if cli_path.contains("copilot") { "copilot --allow-all --no-ask-user" } else { "claude -p" };

    // Write prompt to /tmp/plan-<id>-prompt.md on peer via heredoc
    let prompt_escaped = prompt.replace('\'', "'\\''");
    let write_cmd = format!(
        "printf '%s' '{prompt_escaped}' > /tmp/plan-{plan_id}-prompt.md"
    );
    let out = timeout(
        Duration::from_secs(15),
        tokio::process::Command::new("ssh")
            .args([&cfg.ssh_alias, &write_cmd])
            .output(),
    )
    .await
    .map_err(|_| "prompt write timed out")??;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("prompt write failed: {stderr}").into());
    }

    // Launch in tmux (detached session named plan-<id>)
    let tmux_cmd = format!(
        "tmux new-session -d -s plan-{plan_id} -c /private/tmp/wt-plan-{plan_id} \
        '{cli_binary} -p \"$(cat /tmp/plan-{plan_id}-prompt.md)\"' 2>/dev/null || \
        tmux send-keys -t plan-{plan_id} '' Enter"
    );
    let out = timeout(
        Duration::from_secs(20),
        tokio::process::Command::new("ssh")
            .args([&cfg.ssh_alias, &tmux_cmd])
            .output(),
    )
    .await
    .map_err(|_| "tmux launch timed out")??;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("tmux launch failed: {stderr}").into());
    }
    tracing::info!("delegation: plan {plan_id} launched in tmux:plan-{plan_id} on {}", cfg.ssh_alias);
    Ok(())
}

#[cfg(test)]
#[path = "delegation_pipeline_steps_tests.rs"]
mod tests;
