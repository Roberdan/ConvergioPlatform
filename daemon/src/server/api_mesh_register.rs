// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// POST /api/mesh/register — accept a new peer and add to peers.conf.
// GET /api/mesh/peers-config — return current peers.conf for sync.

use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Peer registration request from a node joining the mesh.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub ssh_alias: String,
    pub user: String,
    pub os: String,
    pub tailscale_ip: String,
    pub dns_name: String,
    pub capabilities: Vec<String>,
    pub role: String,
    #[serde(default)]
    pub lan_ip: Option<String>,
    #[serde(default)]
    pub mac_address: Option<String>,
    #[serde(default)]
    pub gh_account: Option<String>,
}

#[derive(Debug, Serialize)]
struct RegisterResponse {
    ok: bool,
    message: String,
    peers_config: String,
    env_content: String,
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/mesh/register", post(handle_register))
        .route("/api/mesh/peers-config", get(handle_peers_config))
}

fn peers_conf_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home).join(".claude/config/peers.conf")
}

fn env_file_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home).join(".convergio/env")
}

/// POST /api/mesh/register — coordinator accepts a new peer.
#[tracing::instrument(skip_all, fields(peer_name))]
async fn handle_register(
    State(_state): State<ServerState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<Value>, ApiError> {
    tracing::Span::current().record("peer_name", &req.name.as_str());
    tracing::info!(peer = %req.name, ip = %req.tailscale_ip, "mesh register request");

    let path = peers_conf_path();
    let mut registry = load_or_init_registry(&path)?;

    let peer = crate::mesh::peers::PeerConfig {
        ssh_alias: req.ssh_alias,
        user: req.user,
        os: req.os,
        tailscale_ip: req.tailscale_ip,
        dns_name: req.dns_name,
        capabilities: req.capabilities,
        role: req.role.clone(),
        status: "active".to_string(),
        thunderbolt_ip: None,
        lan_ip: req.lan_ip,
        mac_address: req.mac_address,
        gh_account: req.gh_account,
        runners: None,
        runner_paths: None,
    };

    let is_new = !registry.peers.contains_key(&req.name);
    registry.add_peer(&req.name, peer);
    registry.save(&path).map_err(|e| {
        ApiError::internal(format!("failed to save peers.conf: {e}"))
    })?;

    let peers_config = std::fs::read_to_string(&path).unwrap_or_default();
    let env_content = read_env_for_peer();
    let action = if is_new { "registered" } else { "updated" };

    tracing::info!(peer = %req.name, action, "peer {action} in mesh");

    Ok(Json(json!({
        "ok": true,
        "message": format!("peer '{}' {action}", req.name),
        "peers_config": peers_config,
        "env_content": env_content,
    })))
}

/// GET /api/mesh/peers-config — return peers.conf content for sync.
#[tracing::instrument(skip_all)]
async fn handle_peers_config(
    State(_state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let content = std::fs::read_to_string(peers_conf_path())
        .map_err(|e| ApiError::internal(format!("peers.conf read error: {e}")))?;
    Ok(Json(json!({ "ok": true, "peers_config": content })))
}

fn load_or_init_registry(
    path: &std::path::Path,
) -> Result<crate::mesh::peers::PeersRegistry, ApiError> {
    if path.exists() {
        crate::mesh::peers::PeersRegistry::load(path)
            .map_err(|e| ApiError::internal(format!("peers.conf parse error: {e}")))
    } else {
        Ok(crate::mesh::peers::PeersRegistry {
            shared_secret: generate_secret(),
            peers: std::collections::BTreeMap::new(),
        })
    }
}

fn generate_secret() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("convergio-mesh-auto-{ts}")
}

/// Read env file, stripping secrets for safe transfer.
fn read_env_for_peer() -> String {
    let path = env_file_path();
    match std::fs::read_to_string(path) {
        Ok(content) => {
            // Only pass safe keys, redact API keys
            content
                .lines()
                .filter(|l| !l.trim().starts_with('#'))
                .filter(|l| !l.contains("API_KEY") && !l.contains("SECRET"))
                .collect::<Vec<_>>()
                .join("\n")
        }
        Err(_) => String::new(),
    }
}

#[cfg(test)]
#[path = "api_mesh_register_tests.rs"]
mod tests;
