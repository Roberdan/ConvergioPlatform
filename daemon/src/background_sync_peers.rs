// Peer discovery for background sync — extracted from background_sync.rs.
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tracing::{error, info};

use crate::background_sync_http::{
    detect_local_tailscale_ip, peers_conf_path_from_env, resolve_best_addr,
};

/// Query online peers and resolve best reachable address (Thunderbolt -> Tailscale).
/// Returns "host:port" WITHOUT scheme. Fails loud — errors at ERROR level.
pub fn query_active_peers(db: &Arc<Mutex<Connection>>) -> Result<Vec<String>, rusqlite::Error> {
    let conn = db.lock().map_err(|_| {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some("background_sync: DB mutex poisoned".to_string()),
        )
    })?;

    let peers_conf_path = peers_conf_path_from_env();
    let conf_content = match std::fs::read_to_string(&peers_conf_path) {
        Ok(c) => c,
        Err(e) => {
            error!("background_sync: cannot read peers.conf at {peers_conf_path}: {e}");
            return Ok(Vec::new());
        }
    };
    let conf = crate::server::api_mesh::peer_conf::parse_peers_conf(&conf_content);

    let mut stmt = conn.prepare_cached(
        "SELECT DISTINCT peer_name FROM peer_heartbeats \
         WHERE last_seen > unixepoch() - 600",
    )?;
    let online_names: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| match r {
            Ok(v) => Some(v),
            Err(e) => { tracing::warn!("background_sync peer row: {e}"); None }
        })
        .collect();

    if online_names.is_empty() {
        info!("background_sync: no peers with recent heartbeat");
        return Ok(Vec::new());
    }

    let local_ts_ip = detect_local_tailscale_ip();
    let mut addrs = Vec::new();
    for name in &online_names {
        let Some(peer) = conf.get(name.as_str()) else {
            error!("background_sync: peer '{name}' online but NOT in peers.conf");
            continue;
        };
        if let Some(ts_ip) = peer.get("tailscale_ip") {
            if Some(ts_ip.as_str()) == local_ts_ip.as_deref() {
                continue; // skip self
            }
        }
        match resolve_best_addr(name, peer) {
            Some(addr) => {
                info!("background_sync: peer {name} -> {addr}");
                addrs.push(addr);
            }
            None => {
                error!(
                    "background_sync: peer '{name}' has no reachable address — \
                     tried thunderbolt_ip={:?}, tailscale_ip={:?}",
                    peer.get("thunderbolt_ip"),
                    peer.get("tailscale_ip"),
                );
            }
        }
    }
    Ok(addrs)
}
