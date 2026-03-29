/// HTTP transport helpers for background sync.
///
/// Extracted from background_sync.rs to stay under 250-line limit.
/// These functions call peer /api/sync/{export,import} endpoints
/// and handle peer address resolution with multi-transport fallback.
use std::net::TcpStream;
use std::time::Duration;
use tracing::{info, warn};

use crate::db::libsql_adapter::SyncChange;

/// POST local changes to the peer's /api/sync/import endpoint.
pub fn send_changes_to_peer(
    peer_addr: &str,
    changes: &[SyncChange],
) -> Result<(), String> {
    let url = format!("http://{peer_addr}/api/sync/import");
    let payload = serde_json::json!({ "changes": changes });
    let resp = reqwest::blocking::Client::new()
        .post(&url)
        .json(&payload)
        .timeout(Duration::from_secs(10))
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
    let mut url = format!(
        "http://{peer_addr}/api/sync/export?table={table}"
    );
    if let Some(ts) = since {
        url.push_str(&format!("&since={ts}"));
    }
    let resp = reqwest::blocking::Client::new()
        .get(&url)
        .timeout(Duration::from_secs(10))
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
    .filter_map(|(transport, ip)| ip.filter(|s| !s.is_empty()).map(|ip| (transport, ip)))
    .collect();

    for (transport, ip) in &candidates {
        let addr = format!("{ip}:8420");
        match TcpStream::connect_timeout(
            &addr.parse().expect("valid socket addr"),
            Duration::from_secs(2),
        ) {
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

/// Path to peers.conf: CONVERGIO_PEERS_CONF env var → ~/.claude/config/peers.conf.
pub fn peers_conf_path_from_env() -> String {
    std::env::var("CONVERGIO_PEERS_CONF").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{home}/.claude/config/peers.conf")
    })
}
