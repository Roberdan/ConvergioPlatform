/// HTTP transport helpers for background sync.
///
/// Extracted from background_sync.rs to stay under 250-line limit.
/// These functions call peer /api/sync/{export,import} endpoints.
use std::time::Duration;

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
