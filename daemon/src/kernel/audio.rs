// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Mesh audio routing — kernel generates TTS audio, streams to active node, plays locally.
// Active node priority: explicit (cvg kernel here) > last CLI peer > kernel localhost.

use std::path::PathBuf;
use tracing::info;

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
    let node_val: Option<String> = match conn.query_row(
        "SELECT value FROM kernel_config WHERE key = 'active_node'",
        [],
        |r| r.get(0),
    ) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!("kernel.audio: query active_node: {e}");
            None
        }
    };
    let set_at_val: Option<String> = match conn.query_row(
        "SELECT value FROM kernel_config WHERE key = 'active_node_set_at'",
        [],
        |r| r.get(0),
    ) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!("kernel.audio: query active_node_set_at: {e}");
            None
        }
    };

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
    let peer: Option<String> = match conn.query_row(
        "SELECT source FROM kernel_events
         WHERE source != '' AND source != 'localhost' AND source != '127.0.0.1'
         ORDER BY timestamp DESC LIMIT 1",
        [],
        |r| r.get::<_, Option<String>>(0),
    ) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!("kernel.audio: query last_cli_peer: {e}");
            None
        }
    };

    let hostname = peer.filter(|s| !s.is_empty())?;
    info!("kernel.audio: active_node='{}' source=last_cli", hostname);
    Some(ActiveNode { hostname, source: ActiveNodeSource::LastCli })
}

// ----- Local playback --------------------------------------------------------

/// Write WAV bytes to a temp file and exec `afplay` (macOS).
/// The temp file is deleted after playback (or on error).
pub async fn play_local(audio: &[u8]) {
    use tracing::warn;
    let path = temp_wav_path();
    if let Err(e) = std::fs::write(&path, audio) {
        warn!("kernel.audio: could not write temp WAV {}: {e}", path.display());
        return;
    }
    let result = tokio::process::Command::new("afplay")
        .arg(&path)
        .status()
        .await;
    if let Err(e) = std::fs::remove_file(&path) {
        tracing::debug!("kernel.audio: temp file cleanup: {e}");
    } // cleanup regardless of outcome
    match result {
        Ok(s) if s.success() => info!("kernel.audio: afplay complete"),
        Ok(s) => warn!("kernel.audio: afplay exit {:?}", s.code()),
        Err(e) => warn!("kernel.audio: afplay not available: {e}"),
    }
}

pub(crate) fn temp_wav_path() -> PathBuf {
    let pid = std::process::id();
    PathBuf::from(format!("/tmp/cvg_audio_{pid}.wav"))
}

// ----- Tests (external file) -------------------------------------------------

#[cfg(test)]
#[path = "audio_tests.rs"]
mod tests;
