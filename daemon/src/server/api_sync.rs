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
use std::collections::{BTreeMap, BTreeSet};

use crate::db::libsql_adapter::{self, SyncChange};
use crate::server::sync_runtime_status::SyncRuntimeStatusHolder;

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
        .route("/api/sync/status", get(handle_status))
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

#[tracing::instrument(skip_all)]
async fn handle_status(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let runtime = SyncRuntimeStatusHolder::new_daemon_first().snapshot();
    let conn = state.get_conn()?;

    let mut rows = Vec::new();
    let mut meta_error: Option<String> = None;
    match conn.prepare_cached(
        "SELECT peer, table_name, last_sync_at
         FROM _sync_meta
         ORDER BY peer ASC, table_name ASC",
    ) {
        Ok(mut stmt) => {
            let mapped = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            });
            match mapped {
                Ok(iter) => {
                    for item in iter.flatten() {
                        rows.push(item);
                    }
                }
                Err(e) => {
                    meta_error = Some(format!("failed to query _sync_meta rows: {e}"));
                }
            }
        }
        Err(e) => {
            meta_error = Some(format!("_sync_meta unavailable: {e}"));
        }
    }

    let mut per_peer: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut table_peer_sets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut table_last_sync: BTreeMap<String, String> = BTreeMap::new();

    for (peer, table, last_sync_at) in rows {
        per_peer.entry(peer.clone()).or_default().push(json!({
            "table": table,
            "last_sync_at": last_sync_at,
        }));
        table_peer_sets
            .entry(table.clone())
            .or_default()
            .insert(peer);
        let current = table_last_sync.entry(table).or_default();
        if last_sync_at > *current {
            *current = last_sync_at;
        }
    }

    let peers: Vec<Value> = per_peer
        .into_iter()
        .map(|(peer, tables)| {
            json!({
                "peer": peer,
                "table_count": tables.len(),
                "tables": tables,
            })
        })
        .collect();
    let tables: Vec<Value> = table_peer_sets
        .into_iter()
        .map(|(table, peers_for_table)| {
            let latest = table_last_sync.get(&table).cloned();
            json!({
                "table": table,
                "peer_count": peers_for_table.len(),
                "last_sync_at": latest,
            })
        })
        .collect();

    Ok(Json(json!({
        "healthy": runtime.healthy,
        "last_success_at": runtime.last_success_at,
        "last_error": runtime.last_error,
        "transport_mode": runtime.transport_mode,
        "fallback_policy": runtime.fallback_policy,
        "peer_count": peers.len(),
        "table_count": tables.len(),
        "peers": peers,
        "tables": tables,
        "meta_error": meta_error,
    })))
}

#[cfg(test)]
#[path = "api_sync_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "api_sync_status_tests.rs"]
mod status_tests;
