/// Background sync loop — periodically syncs with active mesh peers via HTTP.
///
/// Spawned once at daemon startup. Queries mesh_sync_stats for live peers and
/// uses the timestamp-based `libsql_adapter` to export/import changes over HTTP.
/// Skips ticks silently when the DB lock is contended or no peers are available.
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::background_sync_http::{fetch_changes_from_peer, send_changes_to_peer};
use crate::db::libsql_adapter::{self, SyncMeta};

/// Default sync interval in seconds when CONVERGIO_SYNC_INTERVAL_SECS is unset.
const DEFAULT_INTERVAL_SECS: u64 = 30;

/// Tables eligible for timestamp-based sync. Must have `id` + `updated_at`.
const SYNC_TABLES: &[&str] = &["tasks", "plans", "waves"];

/// Resolve the effective sync interval.
///
/// Priority: explicit `override_secs` arg > CONVERGIO_SYNC_INTERVAL_SECS env var > 30s default.
pub fn resolve_interval_secs(override_secs: Option<u64>) -> u64 {
    if let Some(v) = override_secs {
        return v;
    }
    std::env::var("CONVERGIO_SYNC_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_INTERVAL_SECS)
}

/// Query mesh_sync_stats for peers that are not 'unreachable'.
///
/// Returns an empty Vec — not an error — when the table has no rows or the
/// lock is already held. Callers should skip the tick rather than propagate.
pub fn query_active_peers(db: &Arc<Mutex<Connection>>) -> Result<Vec<String>, rusqlite::Error> {
    let conn = db.lock().map_err(|_| {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some("background_sync: DB mutex poisoned".to_string()),
        )
    })?;

    // Use peer_heartbeats (always exists) — find peers seen in last 10 min.
    // Returns peer URLs like "http://100.x.x.x:8420" for sync HTTP calls.
    let mut stmt = conn.prepare_cached(
        "SELECT DISTINCT peer_name FROM peer_heartbeats \
         WHERE last_seen > unixepoch() - 600"
    )?;
    let names: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    // Resolve peer names to HTTP URLs via peers.conf or tailscale IP
    let mut urls = Vec::new();
    for name in &names {
        // Try to find tailscale_ip from peer_heartbeats load_json
        let ip: Option<String> = conn.query_row(
            "SELECT json_extract(load_json, '$.tailscale_ip') FROM peer_heartbeats WHERE peer_name = ?1",
            [name],
            |row| row.get(0),
        ).ok().flatten();
        if let Some(ip) = ip {
            urls.push(format!("{}:8420", ip));
        }
    }
    if urls.is_empty() && !names.is_empty() {
        debug!("background_sync: {} peers found but no resolved URLs", names.len());
    }
    Ok(urls)
}

/// Sync a single table with a remote peer via HTTP.
///
/// 1. Read local _sync_meta for the last sync timestamp with this peer+table.
/// 2. Export local changes since that timestamp.
/// 3. POST them to the peer's /api/sync/import.
/// 4. GET the peer's /api/sync/export?table=X&since=Y.
/// 5. Apply the remote changes locally.
/// 6. Update _sync_meta with current timestamp.
///
/// Returns the number of remote changes applied, or 0 on failure.
pub fn sync_table_with_peer(
    conn: &Connection,
    peer_addr: &str,
    table: &str,
) -> usize {
    let since = libsql_adapter::get_sync_meta(conn, peer_addr, table)
        .ok()
        .flatten()
        .map(|m| m.last_sync_at);

    // Export local changes and send to peer
    let local_changes = match libsql_adapter::export_changes_since(
        conn,
        table,
        since.as_deref(),
    ) {
        Ok(c) => c,
        Err(e) => {
            warn!(peer = %peer_addr, table, error = %e, "export failed");
            return 0;
        }
    };

    if !local_changes.is_empty() {
        if let Err(e) = send_changes_to_peer(peer_addr, &local_changes) {
            warn!(peer = %peer_addr, error = %e, "send changes failed");
        }
    }

    // Fetch remote changes from peer
    let remote_changes = match fetch_changes_from_peer(
        peer_addr,
        table,
        since.as_deref(),
    ) {
        Ok(c) => c,
        Err(e) => {
            warn!(peer = %peer_addr, table, error = %e, "fetch changes failed");
            return 0;
        }
    };

    let applied = match libsql_adapter::apply_changes(conn, &remote_changes) {
        Ok(n) => n,
        Err(e) => {
            warn!(peer = %peer_addr, table, error = %e, "apply changes failed");
            return 0;
        }
    };

    // Update sync checkpoint
    let now = chrono::Utc::now().to_rfc3339();
    let meta = SyncMeta {
        peer: peer_addr.to_string(),
        table_name: table.to_string(),
        last_sync_at: now,
    };
    if let Err(e) = libsql_adapter::upsert_sync_meta(conn, &meta) {
        warn!(peer = %peer_addr, table, error = %e, "upsert sync meta failed");
    }

    info!(
        peer = %peer_addr,
        table,
        sent = local_changes.len(),
        received = remote_changes.len(),
        applied,
        "background_sync: table sync complete"
    );
    applied
}

/// Spawn the background sync loop.
///
/// Each `interval_secs` tick:
/// 1. Lock the DB to read active peers from mesh_sync_stats.
/// 2. For each peer+table, export/import changes via HTTP.
/// 3. Log results via tracing; skip tick silently on lock contention.
///
/// The returned `JoinHandle` can be aborted to stop the loop.
/// `interval_secs` overrides CONVERGIO_SYNC_INTERVAL_SECS env var when non-zero.
pub fn spawn_sync_loop(
    db: Arc<Mutex<Connection>>,
    interval_secs: u64,
) -> tokio::task::JoinHandle<()> {
    let effective_secs = if interval_secs > 0 {
        interval_secs
    } else {
        resolve_interval_secs(None)
    };

    tokio::spawn(async move {
        info!(
            interval_secs = effective_secs,
            "background_sync: loop spawned, first tick after {}s",
            effective_secs
        );
        let mut ticker = tokio::time::interval(Duration::from_secs(effective_secs));
        // Skip the immediate first tick — let the server finish binding.
        ticker.tick().await;

        loop { // UNBOUNDED: event loop
            ticker.tick().await;

            let peers = match query_active_peers(&db) {
                Ok(p) => p,
                Err(e) => {
                    warn!("background_sync: failed to query peers: {e}");
                    continue;
                }
            };

            if peers.is_empty() {
                debug!("background_sync: no active peers, skipping tick");
                continue;
            }
            info!(
                count = peers.len(),
                "background_sync: syncing with {} peer(s)",
                peers.len()
            );

            // Open a connection to the real dashboard DB for sync operations.
            let db_path = crate::db_path_from_env();
            let conn = match Connection::open(&db_path) {
                Ok(c) => {
                    let _ = c.execute_batch(
                        "PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;",
                    );
                    c
                }
                Err(e) => {
                    warn!("background_sync: cannot open DB at {}: {e}", db_path.display());
                    continue;
                }
            };

            for peer in &peers {
                for table in SYNC_TABLES {
                    sync_table_with_peer(&conn, peer, table);
                }
            }
        }
    })
}

#[cfg(test)]
#[path = "background_sync_tests.rs"]
mod tests;
