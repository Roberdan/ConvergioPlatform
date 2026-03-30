//! Post-sync convergence verification.
//!
//! After each sync round, record the local state checksum in mesh_peer_state
//! and warn if any known peer has diverged for more than 5 minutes.
use rusqlite::Connection;
use tracing::warn;

use crate::server::api_mesh::api_mesh_convergence::compute_local_checksum;

/// Check convergence after a sync round: update local state checksum in
/// mesh_peer_state and warn if any peer has diverged for more than 5 minutes.
///
/// Why: drift detection surfaces replication failures before they cause
/// data-loss incidents across mesh nodes.
pub fn check_convergence(conn: &Connection) {
    let local_checksum = compute_local_checksum(conn);

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // Upsert our own state row, incrementing state_version each time.
    if let Err(e) = conn.execute(
        "INSERT INTO mesh_peer_state (peer_id, state_version, state_checksum, last_seen)
         VALUES (?1, 1, ?2, datetime('now'))
         ON CONFLICT(peer_id) DO UPDATE SET
             state_version = state_version + 1,
             state_checksum = excluded.state_checksum,
             last_seen = excluded.last_seen",
        rusqlite::params![hostname, local_checksum],
    ) {
        warn!("convergence: failed to upsert local state for '{hostname}': {e}");
        return;
    }

    // Warn on peers with a different checksum that last reported > 5 min ago.
    let query = "SELECT peer_id, state_checksum,
                        CAST((julianday('now') - julianday(last_seen)) * 86400 AS INTEGER)
                            AS age_secs
                 FROM mesh_peer_state
                 WHERE peer_id != ?1
                   AND state_checksum != ?2
                   AND (julianday('now') - julianday(last_seen)) * 86400 > 300";

    match conn.prepare(query) {
        Err(e) => {
            warn!("convergence: failed to prepare divergence query: {e}");
        }
        Ok(mut stmt) => {
            let rows = stmt.query_map(
                rusqlite::params![hostname, local_checksum],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?)),
            );
            match rows {
                Err(e) => warn!("convergence: divergence query failed: {e}"),
                Ok(iter) => {
                    for row in iter.flatten() {
                        warn!(
                            "convergence: peer '{}' diverged for {}s \
                             (their checksum: {}, ours: {})",
                            row.0, row.2, row.1, local_checksum
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE plans (id INTEGER PRIMARY KEY, status TEXT);
             CREATE TABLE tasks (id INTEGER PRIMARY KEY, status TEXT);
             CREATE TABLE waves (id INTEGER PRIMARY KEY, status TEXT);
             CREATE TABLE mesh_peer_state (
                 peer_id TEXT PRIMARY KEY,
                 state_version INTEGER NOT NULL DEFAULT 0,
                 state_checksum TEXT NOT NULL DEFAULT '',
                 last_seen TEXT NOT NULL DEFAULT (datetime('now'))
             );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_check_convergence_inserts_local_state() {
        let conn = setup_db();
        check_convergence(&conn);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM mesh_peer_state", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "local state row must be inserted");
    }

    #[test]
    fn test_check_convergence_increments_version() {
        let conn = setup_db();
        check_convergence(&conn);
        check_convergence(&conn);
        let version: i64 = conn
            .query_row(
                "SELECT state_version FROM mesh_peer_state LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(version, 2, "version must increment on second call");
    }

    #[test]
    fn test_check_convergence_no_panic_without_peers() {
        // Should complete silently — no peers to warn about.
        let conn = setup_db();
        check_convergence(&conn);
    }
}
