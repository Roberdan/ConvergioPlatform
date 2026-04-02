/// HTTP transport helpers for background sync.
///
/// Extracted from background_sync.rs to stay under 250-line limit.
/// These functions call peer /api/sync/{export,import} endpoints
/// and handle peer address resolution with multi-transport fallback.
use std::net::TcpStream;
use std::time::Duration;
use tracing::{info, warn};

use crate::db::libsql_adapter::SyncChange;
use crate::mesh::auth::load_shared_secret;

/// Build mesh HMAC auth header: HMAC-SHA256(secret, timestamp+method+path_and_query).
/// Returns (timestamp, hex-encoded signature) or None if no shared secret.
fn mesh_hmac_header(method: &str, path_and_query: &str) -> Option<(String, String)> {
    let conf_path = std::path::PathBuf::from(peers_conf_path_from_env());
    let secret = load_shared_secret(&conf_path)?;
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let message = format!("{timestamp}:{method}:{path_and_query}");
    let sig = crate::mesh::auth::compute_hmac(&secret, message.as_bytes()).ok()?;
    Some((timestamp, hex::encode(sig)))
}

/// Apply mesh HMAC auth headers to a request builder.
fn apply_mesh_auth(
    mut req: reqwest::blocking::RequestBuilder,
    method: &str,
    path_and_query: &str,
) -> reqwest::blocking::RequestBuilder {
    if let Some((ts, sig)) = mesh_hmac_header(method, path_and_query) {
        req = req
            .header("X-Mesh-Timestamp", ts)
            .header("X-Mesh-Signature", sig);
    }
    req
}

/// POST local changes to the peer's /api/sync/import endpoint.
pub fn send_changes_to_peer(
    peer_addr: &str,
    changes: &[SyncChange],
) -> Result<(), String> {
    let path = "/api/sync/import";
    let url = format!("http://{peer_addr}{path}");
    let payload = serde_json::json!({ "changes": changes });
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client build failed: {e}"))?;
    let req = apply_mesh_auth(client.post(&url).json(&payload), "POST", path);
    let resp = req.send().map_err(|e| format!("HTTP POST failed: {e}"))?;
    if !resp.status().is_success() {
    }
    Ok(())
}

/// GET remote changes from the peer's /api/sync/export endpoint.
pub fn fetch_changes_from_peer(
    peer_addr: &str,
    table: &str,
    since: Option<&str>,
) -> Result<Vec<SyncChange>, String> {
    let mut path_query = format!("/api/sync/export?table={table}");
    let mut url = format!("http://{peer_addr}{path_query}");
    if let Some(ts) = since {
        let suffix = format!("&since={ts}");
        url.push_str(&suffix);
        path_query.push_str(&suffix);
    }
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client build failed: {e}"))?;
    let req = apply_mesh_auth(client.get(&url), "GET", &path_query);
    let resp = req.send().map_err(|e| format!("HTTP GET failed: {e}"))?;
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
        ("lan", fields.get("lan_ip").map(|s| s.as_str())),
        ("tailscale", fields.get("tailscale_ip").map(|s| s.as_str())),
    ]
    .into_iter()
    .filter_map(|(transport, ip)| ip.filter(|s| !s.is_empty()).map(|ip| (transport, ip)))
    .collect();

    for (transport, ip) in &candidates {
        let addr = format!("{ip}:8420");
        let sock_addr = match addr.parse() {
            Ok(a) => a,
            Err(e) => {
                warn!("background_sync: peer {name} {transport} bad addr {addr}: {e}");
                continue;
            }
        };
        match TcpStream::connect_timeout(&sock_addr, Duration::from_secs(2)) {
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
    // Allow test/env override to skip system calls in CI or constrained environments.
    if let Ok(ip) = std::env::var("CONVERGIO_LOCAL_TAILSCALE_IP") {
        let ip = ip.trim().to_string();
        if !ip.is_empty() {
            return Some(ip);
        }
    }
    for cmd in &["tailscale", "/Applications/Tailscale.app/Contents/MacOS/Tailscale"] {
        if let Some(ip) = std::process::Command::new(cmd)
            .args(["ip", "-4"]).output().ok() // intentional: local IP detection is best-effort; callers handle None
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok()) // intentional: non-UTF-8 output treated as absent
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

/// Update mesh_sync_stats so the dashboard reflects background sync activity.
/// Bridges the gap between HTTP-based background sync and the mesh stats table.
pub fn update_mesh_sync_stats(
    conn: &rusqlite::Connection,
    peer_addr: &str,
    sent: usize,
    received: usize,
    applied: usize,
    latency_ms: i64,
) {
    let result = conn.execute(
        "INSERT INTO mesh_sync_stats(peer_name, total_sent, total_received, total_applied, \
         last_sync_at, last_latency_ms, last_error) \
         VALUES(?1, ?2, ?3, ?4, strftime('%s','now'), ?5, NULL) \
         ON CONFLICT(peer_name) DO UPDATE SET \
           total_sent = total_sent + excluded.total_sent, \
           total_received = total_received + excluded.total_received, \
           total_applied = total_applied + excluded.total_applied, \
           last_sync_at = excluded.last_sync_at, \
           last_latency_ms = excluded.last_latency_ms, \
           last_error = NULL",
        rusqlite::params![peer_addr, sent as i64, received as i64, applied as i64, latency_ms],
    );
    if let Err(e) = result {
        warn!("background_sync: update mesh_sync_stats failed: {e}");
    }
}
