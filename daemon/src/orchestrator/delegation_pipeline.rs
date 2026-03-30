// Full automated delegation pipeline for `cvg delegation start <plan_id> --peer <peer>`.
// Orchestrates steps: peer config → reachability → rsync binary → rsync repo →
// worktree → plan sync → daemon version check → launch execution.
// Why: plan 706 — delegation was a stub; no automation or traceability.

use super::delegation_pipeline_steps as steps;
use crate::server::api_mesh::peer_conf;
use std::path::Path;

type PipelineError = Box<dyn std::error::Error + Send + Sync>;

/// Run the full automated delegation pipeline for a plan on a named peer.
/// Returns a delegation_id string for monitoring.
///
/// Steps (all automatic, all with timeouts):
///   1. Load peer config from peers.conf
///   2. Verify peer reachable (tailscale ping → SSH fallback)
///   3. Rsync daemon binary to peer
///   4. Rsync repo source (excluding target/, .git/objects, node_modules/, data/)
///   5. Create detached git worktree on peer (NEVER -b)
///   6. Export and sync plan data via daemon API
///   7. Verify peer daemon version; restart if mismatch
///   8. Launch execution in tmux on peer
pub async fn run_full_delegation(
    plan_id: i64,
    peer_name: &str,
    db_path: &Path,
) -> Result<String, PipelineError> {
    tracing::info!("delegation-pipeline: starting plan {plan_id} → peer {peer_name}");

    // Step 1: Load peer config from peers.conf
    let peers_conf_path = peers_conf_path();
    let conf_content = tokio::fs::read_to_string(&peers_conf_path).await.map_err(|e| {
        format!("peers.conf not found at {peers_conf_path}: {e}")
    })?;
    let peers = peer_conf::parse_peers_conf(&conf_content);
    let cfg = steps::load_peer_config(peer_name, &peers)?;

    // Local repo path (env override or default ~/GitHub/ConvergioPlatform)
    let local_repo = super::delegation_core::find_local_repo();

    // Step 2: Verify peer reachable
    tracing::info!("delegation-pipeline: [2/8] verifying peer reachable");
    steps::verify_peer_reachable(&cfg).await.map_err(|e| {
        format!("step 2 (reachability) failed: {e}")
    })?;

    // Step 3: Rsync daemon binary
    tracing::info!("delegation-pipeline: [3/8] rsyncing daemon binary");
    steps::rsync_binary(&cfg, &local_repo).await.map_err(|e| {
        format!("step 3 (rsync binary) failed: {e}")
    })?;

    // Step 4: Rsync repo source
    tracing::info!("delegation-pipeline: [4/8] rsyncing repo source");
    steps::rsync_repo_source(&cfg, &local_repo).await.map_err(|e| {
        format!("step 4 (rsync source) failed: {e}")
    })?;

    // Step 5: Create detached worktree on peer
    tracing::info!("delegation-pipeline: [5/8] creating peer worktree");
    steps::create_peer_worktree(&cfg, plan_id).await.map_err(|e| {
        format!("step 5 (worktree) failed: {e}")
    })?;

    // Step 6: Sync plan data via local daemon API
    tracing::info!("delegation-pipeline: [6/8] syncing plan data");
    sync_plan_to_peer(plan_id, &cfg).await.map_err(|e| {
        format!("step 6 (plan sync) failed: {e}")
    })?;

    // Step 7: Ensure peer daemon is running and at correct version
    tracing::info!("delegation-pipeline: [7/8] checking peer daemon version");
    let local_version = env!("CARGO_PKG_VERSION");
    steps::ensure_peer_daemon(&cfg, local_version).await.map_err(|e| {
        format!("step 7 (daemon version) failed: {e}")
    })?;

    // Step 8: Build prompt and launch execution
    tracing::info!("delegation-pipeline: [8/8] launching execution on peer");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let prompt = super::delegation_core::build_plan_prompt(plan_id, &client).await;
    steps::launch_peer_execution(&cfg, plan_id, &prompt).await.map_err(|e| {
        format!("step 8 (launch) failed: {e}")
    })?;

    // Generate delegation_id for monitoring
    let delegation_id = format!(
        "del-{plan_id}-{peer_name}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );

    // Record in DB via local daemon API
    record_delegation(plan_id, peer_name, &delegation_id, db_path).await;

    tracing::info!("delegation-pipeline: plan {plan_id} fully delegated → {peer_name} ({delegation_id})");
    Ok(delegation_id)
}

/// Step 6 implementation: export plan JSON from local daemon and POST to peer.
async fn sync_plan_to_peer(plan_id: i64, cfg: &steps::PeerConfig) -> Result<(), PipelineError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // Export plan from local daemon
    let export_url = format!("http://localhost:8420/api/plan-db/json/{plan_id}");
    let plan_json = client.get(&export_url).send().await?.text().await?;

    if plan_json.trim().is_empty() || plan_json.contains("\"error\"") {
        return Err(format!("plan {plan_id} export returned empty or error").into());
    }

    // POST to peer daemon
    let peer_ip = if cfg.tailscale_ip.is_empty() { cfg.ssh_alias.clone() } else { cfg.tailscale_ip.clone() };
    let import_url = format!("http://{peer_ip}:8420/api/plan-db/import");
    let resp = client
        .post(&import_url)
        .header("Content-Type", "application/json")
        .body(plan_json)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("plan import to peer failed ({status}): {body}").into());
    }
    tracing::info!("delegation-pipeline: plan {plan_id} synced to peer {}", cfg.name);
    Ok(())
}

/// Record delegation event in local daemon DB. Warns on failure (non-blocking).
async fn record_delegation(plan_id: i64, peer_name: &str, delegation_id: &str, _db_path: &Path) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();
    match client
        .post("http://localhost:8420/api/mesh/delegate")
        .json(&serde_json::json!({
            "plan_id": plan_id,
            "peer": peer_name,
            "delegation_id": delegation_id,
        }))
        .send()
        .await
    {
        Ok(resp) if !resp.status().is_success() => {
            tracing::warn!(
                "delegation-pipeline: failed to record delegation {delegation_id}: HTTP {}",
                resp.status()
            );
        }
        Err(e) => {
            tracing::warn!("delegation-pipeline: failed to record delegation {delegation_id}: {e}");
        }
        _ => {}
    }
}

/// Resolve peers.conf path: CONVERGIO_PEERS_CONF env → ~/.claude/config/peers.conf.
fn peers_conf_path() -> String {
    std::env::var("CONVERGIO_PEERS_CONF").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/.claude/config/peers.conf")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peers_conf_path_uses_env_var() {
        std::env::set_var("CONVERGIO_PEERS_CONF", "/custom/path/peers.conf");
        let path = peers_conf_path();
        assert_eq!(path, "/custom/path/peers.conf");
        std::env::remove_var("CONVERGIO_PEERS_CONF");
    }

    #[test]
    fn peers_conf_path_defaults_to_home() {
        std::env::remove_var("CONVERGIO_PEERS_CONF");
        let path = peers_conf_path();
        assert!(path.ends_with("/.claude/config/peers.conf"));
    }

    #[test]
    fn load_peer_config_missing_peer_errors() {
        use std::collections::HashMap;
        let peers: HashMap<String, HashMap<String, String>> = HashMap::new();
        let result = steps::load_peer_config("nonexistent", &peers);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found in peers.conf"));
    }

    #[test]
    fn load_peer_config_parses_fields() {
        use std::collections::HashMap;
        let mut fields = HashMap::new();
        fields.insert("ssh_alias".to_string(), "mac-m1".to_string());
        fields.insert("tailscale_ip".to_string(), "100.1.2.3".to_string());
        fields.insert("user".to_string(), "roberdan".to_string());
        let mut peers = HashMap::new();
        peers.insert("m1pro".to_string(), fields);
        let cfg = steps::load_peer_config("m1pro", &peers).unwrap();
        assert_eq!(cfg.ssh_alias, "mac-m1");
        assert_eq!(cfg.tailscale_ip, "100.1.2.3");
        assert_eq!(cfg.user, "roberdan");
    }
}
