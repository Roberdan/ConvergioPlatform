/// HTTP transport helpers for background sync.
///
/// Extracted from background_sync.rs to stay under 250-line limit.
/// These functions call peer /api/sync/{export,import} endpoints
/// and handle peer address resolution with multi-transport fallback.
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
use tracing::{error, info, warn};

use crate::db::libsql_adapter::SyncChange;

/// POST local changes to the peer's /api/sync/import endpoint.
pub fn send_changes_to_peer(
    peer_addr: &str,
    changes: &[SyncChange],
) -> Result<(), String> {
    validate_peer_addr(peer_addr)?;
    let url = format!("http://{peer_addr}/api/sync/import");
    let payload = serde_json::json!({ "changes": changes });
    let resp = reqwest::blocking::Client::new()
        .post(&url)
        .json(&payload)
        .timeout(Duration::from_secs(120))
        .send()
        .map_err(|e| format!("HTTP POST failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("peer returned {}", resp.status()));
    }
    Ok(())
}

/// GET remote changes from the peer's /api/sync/export endpoint.
pub fn fetch_changes_from_peer(
    peer_addr: &str,
    table: &str,
    since: Option<&str>,
) -> Result<Vec<SyncChange>, String> {
    validate_peer_addr(peer_addr)?;
    let mut url = format!(
        "http://{peer_addr}/api/sync/export?table={table}"
    );
    if let Some(ts) = since {
        url.push_str(&format!("&since={ts}"));
    }
    let resp = reqwest::blocking::Client::new()
        .get(&url)
        .timeout(Duration::from_secs(120))
        .send()
        .map_err(|e| format!("HTTP GET failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("peer returned {}", resp.status()));
    }
    let body: serde_json::Value = resp
        .json()
        .map_err(|e| format!("JSON parse failed: {e}"))?;
    let changes: Vec<SyncChange> = serde_json::from_value(
        body.get("changes").cloned().unwrap_or_default(),
    )
    .map_err(|e| format!("changes parse failed: {e}"))?;
    Ok(changes)
}

/// Validate peer transport address before HTTP calls.
/// Accepted format: IPv4:port (e.g. 100.64.0.12:8420).
pub fn validate_peer_addr(peer_addr: &str) -> Result<(), String> {
    let trimmed = peer_addr.trim();
    if trimmed.is_empty() {
        return Err("peer address is empty".to_string());
    }
    let socket_addr: SocketAddr = trimmed
        .parse()
        .map_err(|e| format!("peer address parse failed for '{trimmed}': {e}"))?;
    if !socket_addr.ip().is_ipv4() {
        return Err(format!(
            "peer address must be IPv4 host:port, got '{trimmed}'"
        ));
    }
    Ok(())
}

/// Resolve the best reachable address for a peer.
///
/// Priority: Thunderbolt (10.0.0.x) → Tailscale (100.x.x.x).
/// Each candidate is tested with a 2-second TCP connect.
/// Returns "host:port" without scheme.
pub fn resolve_best_addr(
    name: &str,
    fields: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let candidates: Vec<(&str, &str)> = [
        ("thunderbolt", fields.get("thunderbolt_ip").map(|s| s.as_str())),
        ("tailscale", fields.get("tailscale_ip").map(|s| s.as_str())),
    ]
    .into_iter()
    .filter_map(|(transport, ip)| {
        ip.map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|ip| (transport, ip))
    })
    .collect();

    for (transport, ip) in &candidates {
        let addr = format!("{ip}:8420");
        let socket_addr: SocketAddr = match addr.parse() {
            Ok(v) => v,
            Err(e) => {
                error!(
                    "background_sync: peer {name} malformed {transport}_ip '{ip}' \
                     (derived addr '{addr}'): {e}"
                );
                continue;
            }
        };
        match TcpStream::connect_timeout(&socket_addr, Duration::from_secs(2)) {
            Ok(_) => {
                info!("background_sync: peer {name} reachable via {transport} ({addr})");
                return Some(addr);
            }
            Err(e) => {
                warn!("background_sync: peer {name} {transport} ({addr}) unreachable: {e}");
            }
        }
    }
    None
}

/// Detect local Tailscale IP via `tailscale ip -4`. Used to skip self-sync.
pub fn detect_local_tailscale_ip() -> Option<String> {
    if let Ok(ip) = std::env::var("CONVERGIO_LOCAL_TAILSCALE_IP") {
        let ip = ip.trim().to_string();
        if !ip.is_empty() {
            return Some(ip);
        }
    }
    for cmd in &["tailscale", "/Applications/Tailscale.app/Contents/MacOS/Tailscale"] {
        if let Some(ip) = std::process::Command::new(cmd)
            .args(["ip", "-4"]).output().ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            return Some(ip);
        }
    }
    None
}

/// Path to peers.conf: CONVERGIO_PEERS_CONF env var → ~/.claude/config/peers.conf.
pub fn peers_conf_path_from_env() -> String {
    std::env::var("CONVERGIO_PEERS_CONF").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{home}/.claude/config/peers.conf")
    })
}

#[cfg(test)]
#[path = "background_sync_http_tests.rs"]
mod tests;
