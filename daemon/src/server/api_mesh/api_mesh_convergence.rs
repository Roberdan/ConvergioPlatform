//! GET /api/mesh/convergence — per-peer state version, checksum, and drift detection.
use super::super::state::{query_rows, ApiError, ServerState};
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Compute a SHA-256 checksum from key DB table counts and statuses.
/// Hashes: plan count+statuses, task count+statuses, wave count+statuses.
pub(crate) fn compute_local_checksum(conn: &rusqlite::Connection) -> String {
    let mut hasher = Sha256::new();

    let table_queries: &[(&str, &str)] = &[
        ("plans",  "SELECT status, COUNT(*) as c FROM plans GROUP BY status ORDER BY status"),
        ("tasks",  "SELECT status, COUNT(*) as c FROM tasks GROUP BY status ORDER BY status"),
        ("waves",  "SELECT status, COUNT(*) as c FROM waves GROUP BY status ORDER BY status"),
    ];

    for (table, sql) in table_queries {
        hasher.update(table.as_bytes());
        hasher.update(b":");
        let rows = query_rows(conn, sql, []).unwrap_or_default();
        for row in &rows {
            let status = row.get("status").and_then(Value::as_str).unwrap_or("");
            let count = row.get("c").and_then(Value::as_i64).unwrap_or(0);
            hasher.update(format!("{status}={count};").as_bytes());
        }
        hasher.update(b"|");
    }

    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// GET /api/mesh/convergence — mesh-wide convergence status.
///
/// Returns each known peer with its reported state_version, state_checksum,
/// drift from the coordinator's current checksum, and seconds since last_seen.
/// Logs a WARNING for any peer whose last_seen is older than 5 minutes.
#[tracing::instrument(skip_all)]
pub(crate) async fn handle_mesh_convergence(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let coordinator_checksum = compute_local_checksum(conn);

    // Coordinator's monotonic version: epoch seconds at query time.
    let coordinator_version = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let peer_rows = query_rows(
        conn,
        "SELECT peer_id, state_version, state_checksum, last_seen \
         FROM mesh_peer_state ORDER BY peer_id",
        [],
    )?;

    let drift_threshold_secs = 300.0_f64; // 5 minutes
    let mut peers: Vec<Value> = Vec::with_capacity(peer_rows.len());

    for row in &peer_rows {
        let peer_id = row.get("peer_id").and_then(Value::as_str).unwrap_or("");
        let peer_version = row.get("state_version").and_then(Value::as_i64).unwrap_or(0);
        let peer_checksum = row
            .get("state_checksum")
            .and_then(Value::as_str)
            .unwrap_or("");
        let last_seen = row
            .get("last_seen")
            .and_then(Value::as_str)
            .unwrap_or("");

        // Compute seconds since last_seen using SQLite
        let age_secs: f64 = conn
            .query_row(
                "SELECT (julianday('now') - julianday(?1)) * 86400.0 AS age",
                rusqlite::params![last_seen],
                |r| r.get::<_, f64>(0),
            )
            .unwrap_or(f64::MAX);

        let checksum_match = peer_checksum == coordinator_checksum;

        if age_secs > drift_threshold_secs {
            tracing::warn!(
                peer = peer_id,
                age_secs = age_secs as u64,
                "mesh convergence: peer has not reported for >{} seconds (drift alarm)",
                drift_threshold_secs as u64
            );
        }

        peers.push(json!({
            "peer_id": peer_id,
            "state_version": peer_version,
            "state_checksum": peer_checksum,
            "checksum_match": checksum_match,
            "drift_secs": age_secs as i64,
            "last_seen": last_seen,
            "alarm": age_secs > drift_threshold_secs,
        }));
    }

    let converged = peers.iter().all(|p| {
        p.get("checksum_match").and_then(Value::as_bool).unwrap_or(false)
            && !p.get("alarm").and_then(Value::as_bool).unwrap_or(false)
    });

    Ok(Json(json!({
        "ok": true,
        "coordinator": {
            "state_version": coordinator_version,
            "state_checksum": coordinator_checksum,
        },
        "peers": peers,
        "converged": converged,
        "peer_count": peers.len(),
    })))
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
    fn test_checksum_deterministic() {
        let conn = setup_db();
        let c1 = compute_local_checksum(&conn);
        let c2 = compute_local_checksum(&conn);
        assert_eq!(c1, c2);
        assert_eq!(c1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn test_checksum_changes_with_data() {
        let conn = setup_db();
        let c_before = compute_local_checksum(&conn);
        conn.execute("INSERT INTO plans (status) VALUES ('doing')", [])
            .unwrap();
        let c_after = compute_local_checksum(&conn);
        assert_ne!(c_before, c_after);
    }

    #[test]
    fn test_convergence_empty_peers() {
        let conn = setup_db();
        let checksum = compute_local_checksum(&conn);
        assert!(!checksum.is_empty());
        let rows = query_rows(&conn, "SELECT * FROM mesh_peer_state", []).unwrap();
        assert!(rows.is_empty());
    }
}
