/// HTTP sync endpoints for timestamp-based node-to-node sync.
///
/// GET  /api/sync/export?table=<name>&since=<timestamp>  -- export SyncChange[]
/// POST /api/sync/import                                  -- apply SyncChange[]
///
/// These replace the SSH-based crsqlite sync with HTTP transport,
/// using the `libsql_adapter` module for timestamp-based LWW sync.
use super::state::{ApiError, ServerState};
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::db::libsql_adapter::{self, SyncChange};

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub table: Option<String>,
    pub since: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportPayload {
    pub changes: Vec<SyncChange>,
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/sync/export", get(handle_export))
        .route("/api/sync/status", get(handle_sync_status))
        .route(
            "/api/sync/import",
            post(handle_import).layer(
                axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024), // 50 MB for bulk sync
            ),
        )
}

    #[tracing::instrument(skip_all)]
async fn handle_export(
    State(state): State<ServerState>,
    Query(params): Query<ExportQuery>,
) -> Result<Json<Value>, ApiError> {
    let table = params.table.ok_or_else(|| {
        ApiError::bad_request("missing required query param: table")
    })?;

    let conn = state.get_conn()?;
    let changes = libsql_adapter::export_changes_since(
        &conn,
        &table,
        params.since.as_deref(),
    )
    .map_err(|e| {
        ApiError::internal(format!("export failed: {e}"))
    })?;

    Ok(Json(json!({
        "table": table,
        "changes": changes,
        "count": changes.len(),
    })))
}

    #[tracing::instrument(skip_all)]
async fn handle_import(
    State(state): State<ServerState>,
    Json(payload): Json<ImportPayload>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let applied =
        libsql_adapter::apply_changes(&conn, &payload.changes)
            .map_err(|e| {
                ApiError::internal(format!("import failed: {e}"))
            })?;

    Ok(Json(json!({
        "ok": true,
        "applied": applied,
    })))
}

/// GET /api/sync/status — health and policy for background sync.
/// Returns `healthy: true` when at least one peer synced within 5 minutes.
    #[tracing::instrument(skip_all)]
async fn handle_sync_status(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let now: i64 = conn
        .query_row("SELECT strftime('%s','now')", [], |r| r.get(0))
        .unwrap_or(0);
    let threshold = now - 300; // 5 minutes

    let mut stmt = conn
        .prepare_cached(
            "SELECT peer_name, last_sync_at, last_latency_ms, last_error \
             FROM mesh_sync_stats ORDER BY last_sync_at DESC",
        )
        .map_err(|e| ApiError::internal(format!("prepare: {e}")))?;
    let peers: Vec<Value> = stmt
        .query_map([], |row| {
            let last: Option<i64> = row.get(1)?;
            Ok(json!({
                "peer": row.get::<_, String>(0)?,
                "last_sync_at": last,
                "last_sync_ago_s": last.map(|t| now - t),
                "latency_ms": row.get::<_, Option<i64>>(2)?,
                "error": row.get::<_, Option<String>>(3)?,
            }))
        })
        .map_err(|e| ApiError::internal(format!("query: {e}")))?
        .filter_map(|r| r.ok())
        .collect();

    let any_recent = peers.iter().any(|p| {
        p.get("last_sync_at")
            .and_then(Value::as_i64)
            .map_or(false, |t| t > threshold)
    });

    Ok(Json(json!({
        "healthy": any_recent,
        "policy": "daemon-http",
        "interval_secs": crate::background_sync::resolve_interval_secs(None),
        "peers": peers,
    })))
}

#[cfg(test)]
#[path = "api_sync_tests.rs"]
mod tests;
