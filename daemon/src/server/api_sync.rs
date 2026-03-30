// HTTP sync endpoints for timestamp-based node-to-node replication.
// These are the primary sync routes; CRDT (crsqlite) is an optional enhancement.
//
// GET  /api/sync/export?table=<name>&since=<timestamp>  -- export SyncChange[]
// POST /api/sync/import                                  -- apply SyncChange[]
// GET  /api/sync/conflicts                               -- list unresolved sync conflicts
// GET  /api/sync/status                                  -- peer lag, sync coverage
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
        .route("/api/sync/conflicts", get(handle_sync_conflicts))
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

/// GET /api/sync/status — health, peer lag, and sync coverage.
    #[tracing::instrument(skip_all)]
async fn handle_sync_status(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let now: i64 = conn
        .query_row("SELECT strftime('%s','now')", [], |r| r.get(0))
        .unwrap_or(0);
    let threshold = now - 300; // 5 minutes

    let mut stmt_result = conn.prepare_cached(
        "SELECT peer_name, last_sync_at, last_latency_ms, last_error \
         FROM mesh_sync_stats ORDER BY last_sync_at DESC",
    );
    let peers: Vec<Value> = match stmt_result {
        Ok(ref mut stmt) => stmt
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
            .collect(),
        Err(_) => vec![], // mesh_sync_stats may not exist on fresh/minimal nodes
    };

    let any_recent_peer = peers.iter().any(|p| {
        p.get("last_sync_at")
            .and_then(Value::as_i64)
            .map_or(false, |t| t > threshold)
    });

    // Count synced peers/tables from _sync_meta (text-timestamp based)
    let peer_count: i64 = conn
        .query_row("SELECT COUNT(DISTINCT peer) FROM _sync_meta", [], |r| r.get(0))
        .unwrap_or(0);
    let table_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _sync_meta", [], |r| r.get(0))
        .unwrap_or(0);

    // Merge runtime sync status (transport mode, last success/error) from in-memory holder
    let snapshot =
        crate::server::sync_runtime_status::SyncRuntimeStatusHolder::new_daemon_first()
            .snapshot();
    let healthy = any_recent_peer || snapshot.healthy;

    let crdt_tables = crate::db::crdt::required_crdt_tables();
    let crdt_count = crdt_tables.len();

    Ok(Json(json!({
        "healthy": healthy,
        "policy": snapshot.transport_mode,
        "transport_mode": snapshot.transport_mode,
        "fallback_policy": snapshot.fallback_policy,
        "last_success_at": snapshot.last_success_at,
        "last_error": snapshot.last_error,
        "interval_secs": 300u64,
        "http_lww_enabled": true,
        "crdt_tables": crdt_count,
        "crdt_table_list": crdt_tables,
        "peer_count": peer_count,
        "table_count": table_count,
        "peers": peers,
    })))
}

/// GET /api/sync/conflicts — list unresolved CRDT sync conflicts.
    #[tracing::instrument(skip_all)]
async fn handle_sync_conflicts(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;

    // Table may not exist yet on fresh nodes
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_sync_conflicts'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(false);
    if !exists {
        return Ok(Json(json!({ "conflicts": [], "count": 0 })));
    }

    let mut stmt = conn
        .prepare_cached(
            "SELECT id, table_name, pk, local_data, remote_data, source_node, created_at \
             FROM _sync_conflicts WHERE resolved = 0 ORDER BY created_at DESC LIMIT 100",
        )
        .map_err(|e| ApiError::internal(format!("prepare: {e}")))?;
    let conflicts: Vec<Value> = stmt
        .query_map([], |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "table_name": row.get::<_, String>(1)?,
                "pk": row.get::<_, Option<i64>>(2)?,
                "local_data": row.get::<_, Option<String>>(3)?,
                "remote_data": row.get::<_, Option<String>>(4)?,
                "source_node": row.get::<_, Option<String>>(5)?,
                "created_at": row.get::<_, Option<String>>(6)?,
            }))
        })
        .map_err(|e| ApiError::internal(format!("query: {e}")))?
        .filter_map(|r| r.ok())
        .collect();

    let count = conflicts.len();
    Ok(Json(json!({ "conflicts": conflicts, "count": count })))
}

#[cfg(test)]
#[path = "api_sync_tests.rs"]
mod tests;
