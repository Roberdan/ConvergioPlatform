// POST /api/mesh/plan-closed — receive plan closure notification from a mesh peer.
// Cleans up local worktrees and branches matching the completed plan.

use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::process::Command;
use tracing::{info, warn};

#[derive(Debug, Deserialize)]
pub struct PlanClosedRequest {
    pub plan_id: i64,
    #[serde(default)]
    pub plan_name: Option<String>,
    #[serde(default)]
    pub worktree_paths: Vec<String>,
}

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/mesh/plan-closed", post(handle_plan_closed))
}

/// POST /api/mesh/plan-closed — clean up local worktrees for a completed plan.
#[tracing::instrument(skip_all, fields(plan_id))]
async fn handle_plan_closed(
    State(_state): State<ServerState>,
    Json(req): Json<PlanClosedRequest>,
) -> Result<Json<Value>, ApiError> {
    tracing::Span::current().record("plan_id", req.plan_id);
    info!(plan_id = req.plan_id, name = ?req.plan_name, "mesh plan-closed received");

    let mut cleaned = Vec::new();

    // 1. Remove explicitly listed worktree paths
    for path in &req.worktree_paths {
        if let Ok(true) = remove_worktree(path).await {
            cleaned.push(path.clone());
        }
    }

    // 2. Discover and remove worktrees matching the plan ID pattern
    let discovered = discover_plan_worktrees(req.plan_id).await;
    for path in &discovered {
        if cleaned.contains(path) {
            continue;
        }
        if let Ok(true) = remove_worktree(path).await {
            cleaned.push(path.clone());
        }
    }

    // 3. Delete stale branches matching plan pattern
    let deleted_branches = cleanup_plan_branches(req.plan_id).await;

    // 4. Prune worktree metadata
    if let Err(e) = run_git_cmd(&["worktree", "prune"]).await {
        warn!(plan_id = req.plan_id, "worktree prune failed: {e}");
    }

    info!(
        plan_id = req.plan_id, cleaned_count = cleaned.len(),
        branches_deleted = deleted_branches, "plan-closed cleanup complete"
    );

    Ok(Json(json!({
        "ok": true,
        "plan_id": req.plan_id,
        "cleaned": cleaned,
        "branches_deleted": deleted_branches,
    })))
}

/// Remove a single worktree by path. Returns Ok(true) if removed.
async fn remove_worktree(path: &str) -> Result<bool, String> {
    let p = std::path::Path::new(path);
    if !p.exists() { return Ok(false); }
    let out = Command::new("git")
        .args(["worktree", "remove", "--force", path])
        .output().await
        .map_err(|e| format!("git worktree remove failed: {e}"))?;
    if out.status.success() {
        info!("removed worktree {path}");
        return Ok(true);
    }
    warn!("git worktree remove {path}: {}", String::from_utf8_lossy(&out.stderr));
    if p.exists() { tokio::fs::remove_dir_all(p).await.ok(); }
    Ok(p.exists())
}

/// List worktrees whose path contains the plan ID pattern.
async fn discover_plan_worktrees(plan_id: i64) -> Vec<String> {
    let pattern = format!("plan-{plan_id}");
    let out = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output().await;
    let text = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };
    text.lines()
        .filter_map(|l| l.strip_prefix("worktree "))
        .filter(|p| p.contains(&pattern))
        .map(String::from)
        .collect()
}

/// Delete git branches matching worktree-plan-{id} or wt-plan-{id} patterns.
async fn cleanup_plan_branches(plan_id: i64) -> usize {
    let patterns = [
        format!("worktree-plan-{plan_id}"),
        format!("wt-plan-{plan_id}"),
        format!("workspace/ws-plan-{plan_id}"),
    ];
    let out = Command::new("git").args(["branch", "--list"]).output().await;
    let text = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return 0,
    };
    let mut deleted = 0;
    for line in text.lines() {
        let branch = line.trim().trim_start_matches("* ");
        if patterns.iter().any(|p| branch.contains(p))
            && run_git_cmd(&["branch", "-D", branch]).await.is_ok()
        {
            info!("deleted branch {branch}");
            deleted += 1;
        }
    }
    deleted
}

async fn run_git_cmd(args: &[&str]) -> Result<(), String> {
    let out = Command::new("git").args(args).output().await.map_err(|e| e.to_string())?;
    if out.status.success() { Ok(()) }
    else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

/// Broadcast plan-closed notification to all mesh peers.
/// Fire-and-forget: logs errors but never blocks the caller.
pub fn broadcast_plan_closed(plan_id: i64, plan_name: Option<String>, wt_paths: Vec<String>) {
    tokio::spawn(async move {
        broadcast_plan_closed_inner(plan_id, plan_name, wt_paths).await;
    });
}

async fn broadcast_plan_closed_inner(
    plan_id: i64,
    plan_name: Option<String>,
    wt_paths: Vec<String>,
) {
    let conf_path = crate::mesh::peers::peers_conf_path_from_env();
    let conf = std::path::PathBuf::from(&conf_path);
    let registry = match crate::mesh::peers::PeersRegistry::load(&conf) {
        Ok(r) => r,
        Err(e) => {
            warn!("plan-closed broadcast: cannot load peers.conf: {e}");
            return;
        }
    };
    let local_ip = crate::background_sync_http::detect_local_tailscale_ip();
    let payload = json!({
        "plan_id": plan_id,
        "plan_name": plan_name,
        "worktree_paths": wt_paths,
    });
    let body_bytes = serde_json::to_vec(&payload).unwrap_or_default();

    for (name, peer) in registry.list_active() {
        if Some(peer.tailscale_ip.as_str()) == local_ip.as_deref() {
            continue; // skip self
        }
        let addr = format!("{}:8420", peer.tailscale_ip);
        let body_clone = body_bytes.clone();
        let name = name.to_string();
        tokio::spawn(async move {
            let result = post_plan_closed(&addr, &body_clone).await;
            match result {
                Ok(_) => info!("plan-closed sent to peer {name} ({addr})"),
                Err(e) => warn!("plan-closed to peer {name} ({addr}) failed: {e}"),
            }
        });
    }
}

async fn post_plan_closed(peer_addr: &str, body: &[u8]) -> Result<(), String> {
    let url = format!("http://{peer_addr}/api/mesh/plan-closed");
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client build failed: {e}"))?;

    let mut req = client
        .post(&url)
        .header("content-type", "application/json")
        .body(body.to_vec());

    // Apply HMAC auth if shared secret is available
    let conf_path = std::path::PathBuf::from(crate::mesh::peers::peers_conf_path_from_env());
    if let Some(secret) = crate::mesh::auth::load_shared_secret(&conf_path) {
        use sha2::{Sha256, Digest};
        let body_hash = hex::encode(Sha256::digest(body));
        let ts = chrono::Utc::now().timestamp().to_string();
        let msg = format!("{ts}:POST:/api/mesh/plan-closed:{body_hash}");
        if let Ok(sig) = crate::mesh::auth::compute_hmac(&secret, msg.as_bytes()) {
            req = req
                .header("X-Mesh-Timestamp", &ts)
                .header("X-Mesh-Signature", hex::encode(&sig))
                .header("X-Mesh-Body-Hash", &body_hash);
        }
    }

    let resp = req.send().await.map_err(|e| format!("POST failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("peer returned {}", resp.status()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plan_closed_request() {
        let json = r#"{"plan_id": 10055, "plan_name": "test", "worktree_paths": ["/tmp/wt"]}"#;
        let req: PlanClosedRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.plan_id, 10055);
        assert_eq!(req.plan_name.as_deref(), Some("test"));
        assert_eq!(req.worktree_paths, vec!["/tmp/wt"]);
    }

    #[test]
    fn parse_plan_closed_minimal() {
        let json = r#"{"plan_id": 42}"#;
        let req: PlanClosedRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.plan_id, 42);
        assert!(req.plan_name.is_none());
        assert!(req.worktree_paths.is_empty());
    }
}
