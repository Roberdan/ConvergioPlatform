// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// POST /api/mesh/register — accept a new peer and add to peers.conf.
// GET /api/mesh/peers-config — return current peers.conf for sync.

use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
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

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/mesh/register", post(handle_register))
        .route("/api/mesh/peers-config", get(handle_peers_config))
}

fn home_dir() -> Result<std::path::PathBuf, ApiError> {
    dirs::home_dir().ok_or_else(|| ApiError::internal("HOME directory not found".to_string()))
}

fn peers_conf_path() -> Result<std::path::PathBuf, ApiError> {
    Ok(home_dir()?.join(".claude/config/peers.conf"))
}

fn env_file_path() -> Result<std::path::PathBuf, ApiError> {
    Ok(home_dir()?.join(".convergio/env"))
}

/// Reject fields containing INI-breaking or injection characters.
fn validate_field(name: &str, value: &str) -> Result<(), ApiError> {
    if value.is_empty() {
        return Err(ApiError::bad_request(format!("field '{name}' must not be empty")));
    }
    for ch in value.chars() {
        if ch == '\n' || ch == '\r' || ch == '[' || ch == ']' || ch == '=' || ch.is_control() {
            return Err(ApiError::bad_request(format!(
                "field '{name}' contains invalid character"
            )));
        }
    }
    Ok(())
}

/// Stricter validation for IP addresses / DNS names.
fn validate_network_field(name: &str, value: &str) -> Result<(), ApiError> {
    validate_field(name, value)?;
    if !value.chars().all(|c| c.is_ascii_alphanumeric() || ".-:_".contains(c)) {
        return Err(ApiError::bad_request(format!(
            "field '{name}' must contain only alphanumeric, '.', '-', ':', '_'"
        )));
    }
    Ok(())
}

/// Stricter validation for names (no dots, colons).
fn validate_name_field(name: &str, value: &str) -> Result<(), ApiError> {
    validate_field(name, value)?;
    if !value.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)) {
        return Err(ApiError::bad_request(format!(
            "field '{name}' must contain only alphanumeric, '-', '_'"
        )));
    }
    Ok(())
}

fn validate_request(req: &RegisterRequest) -> Result<(), ApiError> {
    validate_name_field("name", &req.name)?;
    validate_name_field("ssh_alias", &req.ssh_alias)?;
    validate_name_field("user", &req.user)?;
    validate_field("os", &req.os)?;
    validate_network_field("tailscale_ip", &req.tailscale_ip)?;
    validate_network_field("dns_name", &req.dns_name)?;
    validate_name_field("role", &req.role)?;
    for cap in &req.capabilities {
        validate_name_field("capabilities", cap)?;
    }
    if let Some(ref lip) = req.lan_ip {
        validate_network_field("lan_ip", lip)?;
    }
    Ok(())
}

/// POST /api/mesh/register — coordinator accepts a new peer.
#[tracing::instrument(skip_all, fields(peer_name))]
async fn handle_register(
    State(_state): State<ServerState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<Value>, ApiError> {
    tracing::Span::current().record("peer_name", &req.name.as_str());
    validate_request(&req)?;
    tracing::info!(peer = %req.name, ip = %req.tailscale_ip, "mesh register request");

    let path = peers_conf_path()?;
    if !path.exists() {
        return Err(ApiError::internal(
            "peers.conf missing; initialize mesh config before registering peers".to_string(),
        ));
    }
    // Serialize concurrent registrations via process-level mutex
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = LOCK.lock().await;

    // F-31: log full error internally; expose only a generic message to callers
    let mut registry = crate::mesh::peers::PeersRegistry::load(&path)
        .map_err(|e| {
            tracing::error!("peers.conf parse error: {e}");
            ApiError::internal("configuration file error".to_string())
        })?;

    let peer = crate::mesh::peers::PeerConfig {
        ssh_alias: req.ssh_alias, user: req.user, os: req.os,
        tailscale_ip: req.tailscale_ip, dns_name: req.dns_name,
        capabilities: req.capabilities, role: req.role.clone(),
        status: "active".to_string(), thunderbolt_ip: None,
        lan_ip: req.lan_ip, mac_address: req.mac_address,
        gh_account: req.gh_account, runners: None, runner_paths: None,
        aliases: vec![],
    };

    let is_new = !registry.peers.contains_key(&req.name);
    registry.add_peer(&req.name, peer);
    registry.save(&path).map_err(|e| {
        tracing::error!("failed to save peers.conf: {e}");
        ApiError::internal("configuration file error".to_string())
    })?;

    let peers_config = std::fs::read_to_string(&path).unwrap_or_default();
    let env_content = read_env_for_peer();
    let action = if is_new { "registered" } else { "updated" };
    tracing::info!(peer = %req.name, %action, "peer {} in mesh", action);

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
    let content = std::fs::read_to_string(peers_conf_path()?)
        .map_err(|e| {
            tracing::error!("peers.conf read error: {e}");
            ApiError::internal("configuration file error".to_string())
        })?;
    Ok(Json(json!({ "ok": true, "peers_config": content })))
}

/// Allowlist of env keys safe to share with joining peers.
const SAFE_ENV_KEYS: &[&str] = &[
    "CONVERGIO_REPO_ROOT", "CONVERGIO_NODE_NAME",
    "RUST_LOG", "RUST_BACKTRACE", "APP_ENV",
];

/// Read env file, only sharing explicitly safe keys.
fn read_env_for_peer() -> String {
    let path = match env_file_path() {
        Ok(p) => p,
        Err(_) => return String::new(),
    };
    match std::fs::read_to_string(path) {
        Ok(content) => content
            .lines()
            .filter(|l| {
                l.split_once('=')
                    .map(|(k, _)| SAFE_ENV_KEYS.contains(&k.trim()))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Err(_) => String::new(),
    }
}

#[cfg(test)]
#[path = "api_mesh_register_tests.rs"]
mod tests;
