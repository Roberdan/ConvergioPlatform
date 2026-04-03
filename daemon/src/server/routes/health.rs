// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Health and telemetry endpoints extracted from routes/mod.rs (250-line limit).

use super::super::api_notify::metrics as notify_metrics;
use super::super::state::ServerState;
use super::super::telemetry;
use axum::extract::State;
use axum::Json;
use std::time::Duration;

/// GET /api/health — cached DB health check (5s TTL).
pub async fn api_health(State(state): State<ServerState>) -> Json<serde_json::Value> {
    static CACHE: std::sync::OnceLock<tokio::sync::Mutex<(std::time::Instant, serde_json::Value)>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| {
        tokio::sync::Mutex::new((
            std::time::Instant::now() - Duration::from_secs(10),
            serde_json::json!({}),
        ))
    });

    let mut guard = cache.lock().await;
    if guard.0.elapsed() < Duration::from_secs(5) {
        let mut cached = guard.1.clone();
        cached["uptime_secs"] = serde_json::json!(state.started_at.elapsed().as_secs());
        return Json(cached);
    }

    let uptime_secs = state.started_at.elapsed().as_secs();
    let conn_result = state.get_conn();
    let db_ok = conn_result.is_ok();
    let (table_count, agent_activity_ok, peer_count) = match conn_result {
        Ok(conn) => {
            let tables = match super::super::state::query_one(
                &conn,
                "SELECT COUNT(*) AS c FROM sqlite_master WHERE type='table'",
                [],
            ) {
                Ok(Some(v)) => v.get("c").and_then(serde_json::Value::as_i64).unwrap_or(0),
                Ok(None) => 0,
                Err(e) => { tracing::warn!("health table count query failed: {e}"); 0 }
            };
            let aa_ok = conn.prepare("SELECT 1 FROM agent_activity LIMIT 0").is_ok();
            let peers =
                super::super::state::query_one(&conn, "SELECT COUNT(*) AS c FROM peer_heartbeats", [])
                    .ok() // intentional: health endpoint remains available even when peer count query fails
                    .flatten()
                    .and_then(|v| v.get("c").and_then(serde_json::Value::as_i64))
                    .unwrap_or(0);
            (tables, aa_ok, peers)
        }
        Err(_) => (0, false, 0),
    };
    let result = serde_json::json!({
        "ok": db_ok && agent_activity_ok,
        "db": db_ok,
        "tables": table_count,
        "agent_activity": agent_activity_ok,
        "peers": peer_count,
        "uptime_secs": uptime_secs,
        "version": env!("CARGO_PKG_VERSION"),
    });
    *guard = (std::time::Instant::now(), result.clone());
    Json(result)
}

/// GET /api/diagnostics/guards — power guard + network watchdog status.
pub async fn api_diagnostics_guards() -> Json<serde_json::Value> {
    let power = crate::power_guard::PowerGuard::status();
    let network_up = crate::network_watchdog::is_network_up();
    Json(serde_json::json!({
        "power_guard": {
            "active": power.active,
            "agent_count": power.agent_count,
            "platform": power.platform,
        },
        "network": {
            "up": network_up,
        },
    }))
}

/// GET /api/telemetry — live request metrics (counters, histograms, error rates).
pub async fn api_telemetry(State(state): State<ServerState>) -> Json<serde_json::Value> {
    let mut snapshot = telemetry::snapshot();
    let notification_delivery = match state.get_conn() {
        Ok(conn) => notify_metrics::telemetry_summary(&conn).unwrap_or_else(|error| {
            tracing::warn!("telemetry notification summary failed: {error}");
            serde_json::json!({
                "total_attempts": 0,
                "successful_attempts": 0,
                "failed_attempts": 0,
                "trace_count": 0,
                "avg_duration_ms": 0,
                "channels": [],
            })
        }),
        Err(error) => {
            tracing::warn!("telemetry connection unavailable: {error}");
            serde_json::json!({
                "total_attempts": 0,
                "successful_attempts": 0,
                "failed_attempts": 0,
                "trace_count": 0,
                "avg_duration_ms": 0,
                "channels": [],
            })
        }
    };
    snapshot["notification_delivery"] = notification_delivery;
    Json(snapshot)
}
