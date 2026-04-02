// HTTP LWW sync loop: timestamp-based peer replication over Tailscale.
// CRDT (crsqlite) is an optional enhancement; this is the primary replication path.
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info, warn};

use crate::background_sync_convergence::check_convergence;
use crate::background_sync_http::{
    fetch_changes_from_peer, send_changes_to_peer, update_mesh_sync_stats,
};
use crate::background_sync_peers::{probe_known_peers, query_active_peers};
use crate::db::libsql_adapter::{self, SyncMeta};

/// Default sync interval in seconds when CONVERGIO_SYNC_INTERVAL_SECS is unset.
const DEFAULT_INTERVAL_SECS: u64 = 30;

/// How many sync ticks between peer probes (10 * 30s = 300s = 5 min).
const PROBE_EVERY_N_TICKS: u64 = 10;

/// Tables eligible for timestamp-based sync. Must have `id INTEGER PRIMARY KEY` + `updated_at`.
/// Note: `projects` has TEXT PK and is NOT sync-eligible; FK constraints are
/// handled by disabling foreign_keys during import (see apply_changes).
const SYNC_TABLES: &[&str] = &["plans", "tasks", "waves", "knowledge_base", "notifications"];

/// Resolve sync interval: explicit arg > env var > 30s default.
pub fn resolve_interval_secs(override_secs: Option<u64>) -> u64 {
    if let Some(v) = override_secs {
        return v;
    }
    match std::env::var("CONVERGIO_SYNC_INTERVAL_SECS") {
        Ok(s) => match s.parse::<u64>() {
            Ok(v) => v,
            Err(_) => DEFAULT_INTERVAL_SECS,
        },
        Err(_) => DEFAULT_INTERVAL_SECS,
    }
}

/// Sync one table with a peer: export local → POST → GET remote → apply → checkpoint.
/// Returns (sent, received, applied) counts; all zero on failure.
pub fn sync_table_with_peer(
    conn: &Connection,
    peer_addr: &str,
    table: &str,
) -> (usize, usize, usize) {
    let since = match libsql_adapter::get_sync_meta(conn, peer_addr, table) {
        Ok(meta) => meta.map(|m| m.last_sync_at),
        Err(e) => {
            tracing::warn!("sync_table get_sync_meta {table}/{peer_addr}: {e}");
            None
        }
    };

    // Export local changes and send to peer
    let local_changes = match libsql_adapter::export_changes_since(
        conn,
        table,
        since.as_deref(),
    ) {
        Ok(c) => c,
        Err(e) => {
            warn!(peer = %peer_addr, table, error = %e, "export failed");
            return (0, 0, 0);
        }
    };

    if !local_changes.is_empty() {
        if let Err(e) = send_changes_to_peer(peer_addr, &local_changes) {
            error!(peer = %peer_addr, error = %e, "send changes failed — aborting sync tick to prevent data loss");
            return (0, 0, 0);
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
            return (0, 0, 0);
        }
    };

    let applied = match libsql_adapter::apply_changes(conn, &remote_changes) {
        Ok(n) => n,
        Err(e) => {
            warn!(peer = %peer_addr, table, error = %e, "apply changes failed");
            return (0, 0, 0);
        }
    };

    // Update sync checkpoint
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let meta = SyncMeta {
        peer: peer_addr.to_string(),
        table_name: table.to_string(),
        last_sync_at: now,
    };
    if let Err(e) = libsql_adapter::upsert_sync_meta(conn, &meta) {
        warn!(peer = %peer_addr, table, error = %e, "upsert sync meta failed");
    }

    let sent = local_changes.len();
    let received = remote_changes.len();
    info!(
        peer = %peer_addr,
        table,
        sent,
        received,
        applied,
        "background_sync: table sync complete"
    );
    (sent, received, applied)
}

/// HTTP LWW sync re-enabled as primary replication path.
/// CRDT over TCP (crsqlite, port 9420) is an optional enhancement, not required.
/// crsqlite C extension is not compiled/deployed on nodes, so this path handles replication.
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
        let mut tick_count: u64 = 0;

        loop {
            ticker.tick().await;
            tick_count += 1;

            // Run ALL sync work on a blocking thread to avoid deadlocking
            // the tokio runtime. reqwest::blocking + rusqlite are both sync.
            let should_probe = tick_count % PROBE_EVERY_N_TICKS == 1;
            let db_clone = db.clone();
            let _ = tokio::task::spawn_blocking(move || {
                // Periodically probe all known peers from peers.conf so
                // peers that come online are discovered without restart.
                if should_probe {
                    probe_known_peers(&db_clone);
                }
                let peers = match query_active_peers(&db_clone) {
                    Ok(p) => p,
                    Err(e) => { warn!("background_sync: query peers: {e}"); return; }
                };
                if peers.is_empty() {
                    error!("background_sync: no reachable peers — sync NOT running");
                    return;
                }
                info!(count = peers.len(), "background_sync: syncing with {} peer(s)", peers.len());
                let db_path = crate::db_path_from_env();
                let conn = match Connection::open(&db_path) {
                    Ok(c) => {
                        let _ = c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;");
                        c
                    }
                    Err(e) => { error!("background_sync: open DB: {e}"); return; }
                };
                for peer in &peers {
                    let t0 = std::time::Instant::now();
                    let (mut total_sent, mut total_recv, mut total_applied) = (0usize, 0, 0);
                    for table in SYNC_TABLES {
                        let (s, r, a) = sync_table_with_peer(&conn, peer, table);
                        total_sent += s;
                        total_recv += r;
                        total_applied += a;
                    }
                    let latency_ms = t0.elapsed().as_millis() as i64;
                    update_mesh_sync_stats(
                        &conn, peer, total_sent, total_recv, total_applied, latency_ms,
                    );
                }
                check_convergence(&conn);
            }).await;
        }
    })
}

#[cfg(test)] #[path = "background_sync_tests.rs"] mod tests;
#[cfg(test)] #[path = "background_sync_tests2.rs"] mod tests2;
