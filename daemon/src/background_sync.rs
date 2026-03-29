/// Background sync loop — syncs with active mesh peers via HTTP (timestamp-based LWW).
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info, warn};

use crate::background_sync_http::{
    detect_local_tailscale_ip, fetch_changes_from_peer, peers_conf_path_from_env,
    resolve_best_addr, send_changes_to_peer,
};
use crate::db::libsql_adapter::{self, SyncMeta};

/// Default sync interval in seconds when CONVERGIO_SYNC_INTERVAL_SECS is unset.
const DEFAULT_INTERVAL_SECS: u64 = 30;

/// Tables eligible for timestamp-based sync. Must have `id` + `updated_at`.
const SYNC_TABLES: &[&str] = &["tasks", "plans", "waves", "knowledge_base", "notifications"];

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

/// Query online peers and resolve best reachable address (Thunderbolt → Tailscale).
/// Returns "host:port" WITHOUT scheme. Fails loud — errors at ERROR level.
pub fn query_active_peers(db: &Arc<Mutex<Connection>>) -> Result<Vec<String>, rusqlite::Error> {
    let conn = db.lock().map_err(|_| {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some("background_sync: DB mutex poisoned".to_string()),
        )
    })?;

    let conf_path = peers_conf_path_from_env();
    let conf_content = std::fs::read_to_string(&conf_path).map_err(|e| {
        error!("background_sync: cannot read {conf_path}: {e}");
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
            Some(format!("peers.conf: {e}")),
        )
    })?;
    let conf = crate::server::api_mesh::peer_conf::parse_peers_conf(&conf_content);
    let mut stmt = conn.prepare_cached(
        "SELECT DISTINCT peer_name FROM peer_heartbeats WHERE last_seen > unixepoch() - 600",
    )?;
    let online_names: Vec<String> = stmt.query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok()).collect();
    if online_names.is_empty() {
        info!("background_sync: no peers with recent heartbeat");
        return Ok(Vec::new());
    }

    let local_ts_ip = detect_local_tailscale_ip();

    let mut addrs = Vec::new();
    for name in &online_names {
        let peer = match conf.get(name.as_str()) {
            Some(p) => p,
            None => {
                error!(
                    "background_sync: peer '{name}' online but NOT in peers.conf"
                );
                continue;
            }
        };
        if let Some(ts_ip) = peer.get("tailscale_ip") {
            if Some(ts_ip.as_str()) == local_ts_ip.as_deref() {
                continue;
            }
        }
        match resolve_best_addr(name, peer) {
            Some(addr) => {
                info!("background_sync: peer {name} → {addr}");
                addrs.push(addr);
            }
            None => {
                error!(
                    "background_sync: peer '{name}' unreachable — \
                     tried thunderbolt_ip={:?}, tailscale_ip={:?}",
                    peer.get("thunderbolt_ip"),
                    peer.get("tailscale_ip"),
                );
            }
        }
    }
    Ok(addrs)
}

/// Sync one table with a peer: export local → POST → GET remote → apply → checkpoint.
/// Returns changes applied, or 0 on failure.
pub fn sync_table_with_peer(
    conn: &Connection,
    peer_addr: &str,
    table: &str,
) -> usize {
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
            return 0;
        }
    };

    if !local_changes.is_empty() {
        if let Err(e) = send_changes_to_peer(peer_addr, &local_changes) {
            error!(peer = %peer_addr, error = %e, "send changes failed — aborting sync tick to prevent data loss");
            return 0;
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

/// Spawn the background sync loop. Abortable via the returned JoinHandle.
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
        ticker.tick().await; // skip first tick — let server finish binding

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
                error!("background_sync: no reachable peers — sync is NOT running");
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
                    if let Err(e) = c.execute_batch(
                        "PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;",
                    ) {
                        tracing::warn!("background_sync: PRAGMA init: {e}");
                    }
                    c
                }
                Err(e) => {
                    error!("background_sync: cannot open DB at {}: {e}", db_path.display());
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
