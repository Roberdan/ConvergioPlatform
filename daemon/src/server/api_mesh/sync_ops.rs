//! Mesh sync status, traffic, and proxy handlers.
use super::super::state::{query_rows, ApiError, ServerState};
use super::peer_conf::{build_ip_name_map, detect_local_node};
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

pub(super) async fn proxy_daemon_get(endpoint: &str) -> Result<Json<Value>, ApiError> {
    let url = format!("http://127.0.0.1:9421/api/{endpoint}");
    let resp = reqwest::get(&url)
        .await
        .map_err(|err| ApiError::internal(format!("daemon request failed: {err}")))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(ApiError::internal(format!(
            "daemon request failed for {endpoint}: HTTP {status}"
        )));
    }
    let body = resp
        .json::<Value>()
        .await
        .map_err(|err| ApiError::internal(format!("invalid daemon JSON for {endpoint}: {err}")))?;
    Ok(Json(body))
}

pub(crate) async fn api_mesh_logs() -> Result<Json<Value>, ApiError> {
    proxy_daemon_get("logs").await
}

pub(crate) async fn api_mesh_metrics() -> Result<Json<Value>, ApiError> {
    proxy_daemon_get("metrics").await
}

pub(crate) async fn api_mesh_sync_stats(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    // Try daemon proxy first, fallback to direct DB query
    if let Ok(result) = proxy_daemon_get("sync-stats").await {
        return Ok(result);
    }
    // Fallback: read from DB directly
    let db = state.open_db()?;
    let rows = query_rows(
        db.connection(),
        "SELECT peer_name, total_sent, total_received, total_applied, \
         last_latency_ms, last_sent_at, last_sync_at FROM mesh_sync_stats",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "peers": rows })))
}

pub(crate) async fn api_mesh_sync_status(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let pending = query_rows(
        &conn,
        "SELECT status, COUNT(*) AS count FROM mesh_events GROUP BY status",
        [],
    )
    .unwrap_or_default();
    let latencies = query_rows(
        &conn,
        "SELECT COALESCE(last_latency_ms,0) AS latency_ms FROM mesh_sync_stats WHERE last_latency_ms IS NOT NULL",
        [],
    )
    .unwrap_or_default();
    let mut samples: Vec<i64> = latencies
        .iter()
        .filter_map(|row| row.get("latency_ms").and_then(Value::as_i64))
        .collect();
    samples.sort_unstable();
    let percentile = |p: f64| -> i64 {
        if samples.is_empty() {
            return 0;
        }
        let idx = ((samples.len() - 1) as f64 * p).round() as usize;
        samples[idx]
    };
    Ok(Json(json!({
        "ok": true,
        "events": pending,
        "latency": {
            "db_sync_p50_ms": percentile(0.50),
            "db_sync_p99_ms": percentile(0.99),
            "targets": {"lan_p50_lt_ms": 10, "wan_p99_lt_ms": 100}
        }
    })))
}

/// Real-time traffic data: per-peer sync counters + heartbeat freshness.
/// Dashboard polls this to drive the mesh flow animation with real data.
pub(crate) async fn api_mesh_traffic(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conf_path = std::env::var("HOME").unwrap_or_default() + "/.claude/config/peers.conf";
    let conf = std::fs::read_to_string(&conf_path).unwrap_or_default();
    let name_map = build_ip_name_map(&conf);
    let local_node = detect_local_node(&conf);

    let rows = query_rows(
        &conn,
        "SELECT peer_name, total_sent, total_received, total_applied, \
         last_sent_at, last_sync_at, last_latency_ms FROM mesh_sync_stats",
        [],
    )
    .unwrap_or_default();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let peers: Vec<Value> = rows
        .iter()
        .map(|r| {
            let raw_name = r.get("peer_name").and_then(Value::as_str).unwrap_or("");
            let friendly = name_map
                .get(raw_name)
                .cloned()
                .unwrap_or_else(|| raw_name.replace(":9420", "").to_string());
            let sent = r.get("total_sent").and_then(Value::as_i64).unwrap_or(0);
            let recv = r.get("total_received").and_then(Value::as_i64).unwrap_or(0);
            let last_sync = r.get("last_sync_at").and_then(Value::as_i64).unwrap_or(0);
            let latency = r
                .get("last_latency_ms")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            json!({
                "peer": friendly,
                "total_sent": sent,
                "total_received": recv,
                "total_applied": r.get("total_applied").and_then(Value::as_i64).unwrap_or(0),
                "last_sync_ago_s": if last_sync > 0 { now - last_sync } else { -1 },
                "latency_ms": latency,
                "active": last_sync > 0 && (now - last_sync) < 30
            })
        })
        .collect();

    let hb_rows = query_rows(
        &conn,
        "SELECT peer_name, last_seen FROM peer_heartbeats WHERE peer_name IS NOT NULL AND peer_name != '' AND peer_name NOT LIKE '%.%.%.%:%'",
        [],
    ).unwrap_or_default();
    let heartbeats: Vec<Value> = hb_rows
        .iter()
        .map(|r| {
            let name = r.get("peer_name").and_then(Value::as_str).unwrap_or("");
            let last = r.get("last_seen").and_then(Value::as_i64).unwrap_or(0);
            json!({ "peer": name, "last_seen_ago_s": if last > 0 { now - last } else { -1 } })
        })
        .collect();

    Ok(Json(json!({
        "ok": true,
        "local_node": local_node,
        "ts": now,
        "sync_peers": peers,
        "heartbeats": heartbeats
    })))
}
