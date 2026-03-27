// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Mesh audio routing — kernel generates TTS audio, streams to active node, plays locally.
// Active node priority: explicit (cvg kernel here) > last CLI peer > kernel localhost.

use std::path::PathBuf;
use tracing::{info, warn};

// ----- Active node resolution ------------------------------------------------

/// Origin of the active-node determination (for diagnostics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveNodeSource {
    /// Set explicitly via `cvg kernel here` (valid 8 h).
    Explicit,
    /// Inferred from the most recent peer that sent a kernel event.
    LastCli,
    /// Fallback: play locally on the kernel node.
    Localhost,
}

/// Resolved active node: a hostname reachable over mesh HTTP (:8420).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveNode {
    /// Hostname or "localhost".
    pub hostname: String,
    pub source: ActiveNodeSource,
}

impl ActiveNode {
    /// Returns true when audio should be played locally (no HTTP hop).
    pub fn is_local(&self) -> bool {
        let h = self.hostname.to_lowercase();
        h == "localhost" || h == "127.0.0.1" || h == "::1"
    }

    /// HTTP base URL for remote play.
    pub fn base_url(&self) -> String {
        format!("http://{}:8420", self.hostname)
    }
}

// ----- Node detection --------------------------------------------------------

/// Determine the active audio target node from the database.
///
/// Priority:
/// 1. `kernel_config` key `active_node` — set by `cvg kernel here`.
///    Valid only if `active_node_set_at` is within 8 hours.
/// 2. Most recent entry in `kernel_events` where `source` is a peer hostname
///    (peers log events with their hostname as source).
/// 3. Fallback: localhost.
///
/// `conn` is a live rusqlite connection.
pub fn resolve_active_node(conn: &rusqlite::Connection) -> ActiveNode {
    // Priority 1 — explicit kernel_config active_node (valid 8 h = 28800 s)
    if let Some(node) = query_explicit_node(conn) {
        return node;
    }
    // Priority 2 — last CLI peer from kernel_events
    if let Some(node) = query_last_cli_peer(conn) {
        return node;
    }
    // Priority 3 — fallback
    ActiveNode { hostname: "localhost".to_string(), source: ActiveNodeSource::Localhost }
}

fn query_explicit_node(conn: &rusqlite::Connection) -> Option<ActiveNode> {
    // Read active_node + active_node_set_at from kernel_config
    let node_val: Option<String> = conn
        .query_row(
            "SELECT value FROM kernel_config WHERE key = 'active_node'",
            [],
            |r| r.get(0),
        )
        .ok();
    let set_at_val: Option<String> = conn
        .query_row(
            "SELECT value FROM kernel_config WHERE key = 'active_node_set_at'",
            [],
            |r| r.get(0),
        )
        .ok();

    let node = node_val.filter(|s| !s.is_empty())?;
    // Validate 8 h window using sqlite datetime comparison
    if let Some(set_at) = set_at_val {
        let is_fresh: bool = conn
            .query_row(
                "SELECT (strftime('%s','now') - strftime('%s', ?1)) < 28800",
                rusqlite::params![set_at],
                |r| r.get::<_, bool>(0),
            )
            .unwrap_or(false);
        if !is_fresh {
            info!("kernel.audio: explicit active_node '{}' expired (>8h)", node);
            return None;
        }
    }
    info!("kernel.audio: active_node='{}' source=explicit", node);
    Some(ActiveNode { hostname: node, source: ActiveNodeSource::Explicit })
}

fn query_last_cli_peer(conn: &rusqlite::Connection) -> Option<ActiveNode> {
    // kernel_events.source stores the originating hostname for peer-originated events.
    // Pick the most recent non-empty source that isn't 'localhost'/'127.0.0.1'.
    let peer: Option<String> = conn
        .query_row(
            "SELECT source FROM kernel_events
             WHERE source != '' AND source != 'localhost' AND source != '127.0.0.1'
             ORDER BY timestamp DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .ok()
        .flatten();

    let hostname = peer.filter(|s| !s.is_empty())?;
    info!("kernel.audio: active_node='{}' source=last_cli", hostname);
    Some(ActiveNode { hostname, source: ActiveNodeSource::LastCli })
}

// ----- Play ------------------------------------------------------------------

/// Play `audio` (raw WAV bytes) on the active node.
///
/// - If the active node is local: write to a temp file, exec `afplay`, delete.
/// - If the active node is remote: HTTP POST to `{base_url}/api/kernel/play`.
///
/// Errors are logged but never fatal — audio is best-effort.
pub async fn play_on_active_node(audio: &[u8], conn: &rusqlite::Connection) {
    let node = resolve_active_node(conn);
    info!(
        "kernel.audio: play {} bytes on '{}' (source={:?})",
        audio.len(),
        node.hostname,
        node.source
    );
    if node.is_local() {
        play_local(audio).await;
    } else {
        play_remote(audio, &node.base_url()).await;
    }
}

/// Write WAV bytes to a temp file and exec `afplay` (macOS).
/// The temp file is deleted after playback (or on error).
pub async fn play_local(audio: &[u8]) {
    let path = temp_wav_path();
    if let Err(e) = std::fs::write(&path, audio) {
        warn!("kernel.audio: could not write temp WAV {}: {e}", path.display());
        return;
    }
    let result = tokio::process::Command::new("afplay")
        .arg(&path)
        .status()
        .await;
    let _ = std::fs::remove_file(&path); // cleanup regardless of outcome
    match result {
        Ok(s) if s.success() => info!("kernel.audio: afplay complete"),
        Ok(s) => warn!("kernel.audio: afplay exit {:?}", s.code()),
        Err(e) => warn!("kernel.audio: afplay not available: {e}"),
    }
}

/// POST WAV bytes to `/api/kernel/play` on a remote mesh node.
async fn play_remote(audio: &[u8], base_url: &str) {
    let url = format!("{base_url}/api/kernel/play");
    info!("kernel.audio: POST {} bytes to {url}", audio.len());
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("kernel.audio: reqwest build error: {e}");
            return;
        }
    };
    match client
        .post(&url)
        .header("content-type", "audio/wav")
        .body(audio.to_vec())
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            info!("kernel.audio: remote play accepted status={}", resp.status())
        }
        Ok(resp) => warn!("kernel.audio: remote play rejected status={}", resp.status()),
        Err(e) => warn!("kernel.audio: remote play HTTP error: {e}"),
    }
}

fn temp_wav_path() -> PathBuf {
    let pid = std::process::id();
    PathBuf::from(format!("/tmp/cvg_audio_{pid}.wav"))
}

// ----- Tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE kernel_config (
                 key TEXT PRIMARY KEY NOT NULL,
                 value TEXT NOT NULL DEFAULT '',
                 updated_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE kernel_events (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                 severity TEXT NOT NULL DEFAULT 'ok',
                 source TEXT NOT NULL DEFAULT '',
                 message TEXT NOT NULL DEFAULT '',
                 action_taken TEXT NOT NULL DEFAULT ''
             );",
        )
        .expect("schema");
        conn
    }

    #[test]
    fn fallback_to_localhost_when_no_data() {
        let conn = in_memory_db();
        let node = resolve_active_node(&conn);
        assert_eq!(node.hostname, "localhost");
        assert_eq!(node.source, ActiveNodeSource::Localhost);
    }

    #[test]
    fn explicit_node_fresh() {
        let conn = in_memory_db();
        // Insert explicit active_node set 1 minute ago
        conn.execute_batch(
            "INSERT INTO kernel_config(key, value) VALUES ('active_node', 'macM5Max');
             INSERT INTO kernel_config(key, value) VALUES
               ('active_node_set_at', datetime('now', '-1 minute'));",
        )
        .unwrap();
        let node = resolve_active_node(&conn);
        assert_eq!(node.hostname, "macM5Max");
        assert_eq!(node.source, ActiveNodeSource::Explicit);
        assert!(!node.is_local());
    }

    #[test]
    fn explicit_node_expired_falls_back() {
        let conn = in_memory_db();
        // Insert an expired active_node (10 hours ago)
        conn.execute_batch(
            "INSERT INTO kernel_config(key, value) VALUES ('active_node', 'macM5Max');
             INSERT INTO kernel_config(key, value) VALUES
               ('active_node_set_at', datetime('now', '-10 hours'));",
        )
        .unwrap();
        let node = resolve_active_node(&conn);
        // Should NOT return macM5Max — expired
        assert_ne!(node.hostname, "macM5Max");
    }

    #[test]
    fn last_cli_peer_detected() {
        let conn = in_memory_db();
        conn.execute_batch(
            "INSERT INTO kernel_events(source, message) VALUES ('macProM1', 'cvg task update');",
        )
        .unwrap();
        let node = resolve_active_node(&conn);
        assert_eq!(node.hostname, "macProM1");
        assert_eq!(node.source, ActiveNodeSource::LastCli);
    }

    #[test]
    fn explicit_takes_priority_over_last_cli() {
        let conn = in_memory_db();
        conn.execute_batch(
            "INSERT INTO kernel_config(key, value) VALUES ('active_node', 'macM5Max');
             INSERT INTO kernel_config(key, value) VALUES
               ('active_node_set_at', datetime('now', '-1 minute'));
             INSERT INTO kernel_events(source, message) VALUES ('macProM1', 'cvg task update');",
        )
        .unwrap();
        let node = resolve_active_node(&conn);
        assert_eq!(node.hostname, "macM5Max");
        assert_eq!(node.source, ActiveNodeSource::Explicit);
    }

    #[test]
    fn localhost_is_local() {
        let node =
            ActiveNode { hostname: "localhost".to_string(), source: ActiveNodeSource::Localhost };
        assert!(node.is_local());
    }

    #[test]
    fn remote_node_is_not_local() {
        let node =
            ActiveNode { hostname: "macM5Max".to_string(), source: ActiveNodeSource::LastCli };
        assert!(!node.is_local());
        assert_eq!(node.base_url(), "http://macM5Max:8420");
    }

    #[test]
    fn temp_wav_path_contains_pid() {
        let path = temp_wav_path();
        let pid = std::process::id();
        assert!(path.to_string_lossy().contains(&pid.to_string()));
    }
}
