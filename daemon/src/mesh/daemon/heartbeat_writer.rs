// Heartbeat writer: persists node heartbeat with version metadata.

use std::sync::LazyLock;

/// Cached rustc version string, resolved once at first use.
static RUSTC_VERSION: LazyLock<String> = LazyLock::new(|| {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
});

/// Write a heartbeat row for `node_name` with system load and version info.
pub fn write_heartbeat(
    conn: &rusqlite::Connection,
    node_name: &str,
    load: &serde_json::Value,
    ts: u64,
) -> Result<(), rusqlite::Error> {
    let load_json = serde_json::to_string(load).unwrap_or_default();
    let version = env!("CARGO_PKG_VERSION");
    let rustc = &*RUSTC_VERSION;
    conn.execute(
        "INSERT OR REPLACE INTO peer_heartbeats \
         (peer_name, last_seen, load_json, version, rustc_version) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![node_name, ts, load_json, version, rustc],
    )?;
    Ok(())
}
