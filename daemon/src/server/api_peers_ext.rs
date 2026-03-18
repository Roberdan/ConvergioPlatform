use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/peers/coordinator", get(handle_coordinator))
        .route("/api/mesh/topology", get(handle_topology))
        .route("/api/mesh/ping/:peer", get(handle_ping))
        .route("/api/mesh/diagnostics", get(handle_diagnostics))
}

/// GET /api/peers/coordinator — return current coordinator node
async fn handle_coordinator(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let peers = query_rows(
        conn,
        "SELECT peer_name, last_seen, load_json, capabilities \
         FROM peer_heartbeats ORDER BY last_seen DESC",
        [],
    )?;

    // Coordinator is the peer with "coordinator" or "mac-worker-2" in name,
    // or the one with the most recent heartbeat if no explicit coordinator
    let coordinator = peers.iter().find(|p| {
        let name = p.get("peer_name").and_then(Value::as_str).unwrap_or("");
        name.contains("mac-worker-2") || name.contains("coordinator")
    });

    let coord_info = match coordinator {
        Some(c) => {
            let seen = c.get("last_seen").and_then(Value::as_f64).unwrap_or(0.0);
            json!({
                "ok": true,
                "coordinator": c,
                "is_online": now_secs - seen < 120.0,
            })
        }
        None => json!({
            "ok": true,
            "coordinator": null,
            "is_online": false,
            "message": "no coordinator found",
        }),
    };

    Ok(Json(coord_info))
}

/// GET /api/mesh/topology — active connections graph
async fn handle_topology(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let peers = query_rows(
        conn,
        "SELECT peer_name, last_seen, load_json, capabilities \
         FROM peer_heartbeats ORDER BY peer_name",
        [],
    )?;

    let nodes: Vec<Value> = peers
        .iter()
        .map(|p| {
            let name = p.get("peer_name").and_then(Value::as_str).unwrap_or("");
            let seen = p.get("last_seen").and_then(Value::as_f64).unwrap_or(0.0);
            let role = if name.contains("mac-worker-2") || name.contains("coordinator") {
                "coordinator"
            } else {
                "worker"
            };
            json!({
                "name": name,
                "role": role,
                "online": now_secs - seen < 120.0,
                "last_seen": seen,
            })
        })
        .collect();

    // Build edges from sync stats if available
    let edges = query_rows(
        conn,
        "SELECT peer_name, avg_latency_ms, last_sync_at \
         FROM mesh_sync_stats ORDER BY peer_name",
        [],
    )
    .unwrap_or_default();

    let local_host = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let edge_list: Vec<Value> = edges
        .iter()
        .map(|e| {
            let peer = e.get("peer_name").and_then(Value::as_str).unwrap_or("");
            let latency = e
                .get("avg_latency_ms")
                .and_then(Value::as_f64)
                .unwrap_or(-1.0);
            json!({
                "from": local_host,
                "to": peer,
                "latency_ms": latency,
            })
        })
        .collect();

    Ok(Json(json!({
        "ok": true,
        "nodes": nodes,
        "edges": edge_list,
    })))
}

/// GET /api/mesh/ping/:peer — measure RTT to peer
async fn handle_ping(
    State(state): State<ServerState>,
    Path(peer): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    // Check if peer exists
    let peer_info = query_one(
        conn,
        "SELECT peer_name, last_seen FROM peer_heartbeats WHERE peer_name = ?1",
        rusqlite::params![peer],
    )?;

    if peer_info.is_none() {
        return Err(ApiError::bad_request(format!("peer '{peer}' not found")));
    }

    // Try TCP connection to peer's mesh port (9420)
    let start = std::time::Instant::now();
    let addr = format!("{peer}:9420");
    let timeout = std::time::Duration::from_secs(5);

    let result = tokio::time::timeout(timeout, tokio::net::TcpStream::connect(&addr)).await;

    let (reachable, latency_ms) = match result {
        Ok(Ok(_stream)) => (true, start.elapsed().as_secs_f64() * 1000.0),
        Ok(Err(_)) | Err(_) => (false, -1.0),
    };

    Ok(Json(json!({
        "ok": true,
        "peer": peer,
        "reachable": reachable,
        "latency_ms": latency_ms,
    })))
}

/// GET /api/mesh/diagnostics — overall mesh health
async fn handle_diagnostics(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let total_peers = query_one(conn, "SELECT COUNT(*) AS c FROM peer_heartbeats", [])?
        .and_then(|v| v.get("c").and_then(Value::as_i64))
        .unwrap_or(0);

    let online_peers = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM peer_heartbeats WHERE ?1 - last_seen < 120",
        rusqlite::params![now_secs],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    let mut warnings = Vec::new();
    if online_peers == 0 && total_peers > 0 {
        warnings.push("All peers offline".to_string());
    }

    let uptime = state.started_at.elapsed().as_secs();

    Ok(Json(json!({
        "ok": true,
        "total_peers": total_peers,
        "online_peers": online_peers,
        "uptime_secs": uptime,
        "version": env!("CARGO_PKG_VERSION"),
        "warnings": warnings,
    })))
}

#[cfg(test)]
mod tests {
    use crate::db::PlanDb;
    use crate::server::state::{query_one, query_rows};

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE peer_heartbeats (
                     peer_name TEXT PRIMARY KEY, last_seen REAL,
                     load_json TEXT, capabilities TEXT
                 );
                 CREATE TABLE mesh_sync_stats (
                     peer_name TEXT PRIMARY KEY, avg_latency_ms REAL,
                     last_sync_at TEXT
                 );
                 INSERT INTO peer_heartbeats VALUES
                     ('mac-worker-2', strftime('%s','now'), '{\"cpu\":20}', 'coordinator'),
                     ('linux-worker', strftime('%s','now') - 60, '{\"cpu\":50}', 'worker'),
                     ('mac-worker-1', strftime('%s','now') - 200, '{\"cpu\":30}', 'worker');",
            )
            .expect("schema");
        db
    }

    #[test]
    fn api_peers_coordinator_found() {
        let db = setup_db();
        let peers = query_rows(
            db.connection(),
            "SELECT peer_name FROM peer_heartbeats ORDER BY last_seen DESC",
            [],
        )
        .unwrap();
        let coordinator = peers.iter().find(|p| {
            p.get("peer_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .contains("mac-worker-2")
        });
        assert!(coordinator.is_some());
    }

    #[test]
    fn api_peers_topology_nodes() {
        let db = setup_db();
        let nodes = query_rows(
            db.connection(),
            "SELECT peer_name FROM peer_heartbeats ORDER BY peer_name",
            [],
        )
        .unwrap();
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn api_peers_diagnostics_counts() {
        let db = setup_db();
        let total: i64 = db
            .connection()
            .query_row("SELECT COUNT(*) FROM peer_heartbeats", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 3);
    }
}
