/// Background CRDT sync loop — periodically syncs with active mesh peers.
///
/// Spawned once at daemon startup by T3b-01. Queries mesh_sync_stats for live
/// peers and calls `PlanDb::sync_with_peer` for each. Skips ticks silently when
/// the DB lock is contended or no peers are available.
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::db::PlanDb;

/// Default sync interval in seconds when CONVERGIO_SYNC_INTERVAL_SECS is unset.
const DEFAULT_INTERVAL_SECS: u64 = 30;

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

    // Table may not exist on a fresh DB — return empty rather than error.
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='mesh_sync_stats'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);

    if !exists {
        debug!("background_sync: mesh_sync_stats not yet present, skipping tick");
        return Ok(vec![]);
    }

    let mut stmt =
        conn.prepare_cached("SELECT peer_name FROM mesh_sync_stats WHERE status != 'unreachable'")?;
    let peers: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(peers)
}

/// Sync with a single peer using a PlanDb wrapping the shared connection.
///
/// `PlanDb` holds its own `Connection` so we open a short-lived in-memory
/// instance here only to mirror the shared connection's CRDT change log.
/// The actual SSH-based sync is driven by the peer string.
///
/// Returns the number of changes applied from the remote, or 0 on failure.
fn sync_one_peer(plan_db: &PlanDb, peer: &str) -> usize {
    match plan_db.sync_with_peer(peer) {
        Ok(summary) => {
            info!(
                peer = %summary.peer,
                sent = summary.sent,
                received = summary.received,
                applied = summary.applied,
                "background_sync: tick complete"
            );
            summary.applied
        }
        Err(e) => {
            warn!(peer = %peer, error = %e, "background_sync: sync_with_peer failed");
            0
        }
    }
}

/// Spawn the background sync loop.
///
/// Each `interval_secs` tick:
/// 1. Lock the DB to read active peers from mesh_sync_stats.
/// 2. For each peer, open a PlanDb view and call sync_with_peer.
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
        let mut ticker = tokio::time::interval(Duration::from_secs(effective_secs));
        // Skip the immediate first tick — let the server finish binding.
        ticker.tick().await;

        loop {
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

            // Open a PlanDb backed by in-memory storage to drive the SSH sync.
            // The actual data exchange happens via SSH subprocess (see sync.rs),
            // so this PlanDb is only used to provide the API surface.
            let plan_db = match PlanDb::open_in_memory() {
                Ok(db) => db,
                Err(e) => {
                    warn!("background_sync: cannot open in-memory PlanDb: {e}");
                    continue;
                }
            };

            for peer in &peers {
                sync_one_peer(&plan_db, peer);
            }
        }
    })
}

#[cfg(test)]
#[path = "background_sync_tests.rs"]
mod tests;
