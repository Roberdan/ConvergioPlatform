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
        .route(
            "/api/sync/import",
            post(handle_import).layer(
                axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024), // 50 MB
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

#[cfg(test)]
#[path = "api_sync_tests.rs"]
mod tests;
