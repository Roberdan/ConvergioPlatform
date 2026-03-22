// api_deliverables: router + list/get/create handlers
// approve/version handlers → api_deliverables_handlers.rs
use super::api_deliverables_handlers::{approve_deliverable, version_deliverable};
use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/deliverables", get(list_deliverables).post(create_deliverable))
        .route("/api/deliverables/:id", get(get_deliverable))
        .route("/api/deliverables/:id/approve", post(approve_deliverable))
        .route("/api/deliverables/:id/version", post(version_deliverable))
}

async fn list_deliverables(
    State(state): State<ServerState>,
    Query(qs): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(v) = qs.get("project_id").filter(|v| !v.is_empty()) {
        conditions.push("project_id=?".to_string());
        params.push(v.clone());
    }
    if let Some(v) = qs.get("task_id").filter(|v| !v.is_empty()) {
        conditions.push("task_id=?".to_string());
        params.push(v.clone());
    }
    if let Some(v) = qs.get("status").filter(|v| !v.is_empty()) {
        conditions.push("status=?".to_string());
        params.push(v.clone());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, task_id, project_id, name, output_type, output_path, status, \
         version, approved_by, approved_at, created_at, updated_at \
         FROM deliverables{where_clause} ORDER BY id DESC"
    );

    let rows = conn
        .prepare(&sql)
        .and_then(|mut stmt| {
            let idx: Vec<&dyn rusqlite::ToSql> =
                params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
            stmt.query_map(idx.as_slice(), |row| row_to_json(row))
                .and_then(|mapped| mapped.collect::<Result<Vec<_>, _>>())
        })
        .map_err(|e| ApiError::internal(format!("list deliverables failed: {e}")))?;

    Ok(Json(Value::Array(rows)))
}

pub async fn get_deliverable(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let row = query_one(
        &conn,
        "SELECT id, task_id, project_id, name, output_type, output_path, status, \
         version, approved_by, approved_at, created_at, updated_at \
         FROM deliverables WHERE id=?1",
        rusqlite::params![id],
    )?;

    let mut deliverable =
        row.ok_or_else(|| ApiError::bad_request(format!("deliverable {id} not found")))?;

    // Attach on-disk metadata.json if the output_path exists
    if let Some(path) = deliverable.get("output_path").and_then(Value::as_str) {
        let meta_path = std::path::Path::new(path).join("metadata.json");
        if meta_path.is_file() {
            if let Ok(raw) = fs::read_to_string(&meta_path) {
                if let Ok(meta) = serde_json::from_str::<Value>(&raw) {
                    if let Some(obj) = deliverable.as_object_mut() {
                        obj.insert("metadata".to_string(), meta);
                    }
                }
            }
        }
    }

    Ok(Json(deliverable))
}

#[derive(Deserialize)]
struct CreateBody {
    task_id: Option<i64>,
    project_id: String,
    name: String,
    output_type: String,
    #[serde(default)]
    agent: Option<String>,
}

async fn create_deliverable(
    State(state): State<ServerState>,
    Json(body): Json<CreateBody>,
) -> Result<Json<Value>, ApiError> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::bad_request("name is required"));
    }
    let project_id = body.project_id.trim().to_string();
    if project_id.is_empty() {
        return Err(ApiError::bad_request("project_id is required"));
    }

    let now = Utc::now();
    let date_prefix = now.format("%Y-%m-%d").to_string();
    let folder_name = format!("{date_prefix}_{name}_v1");

    // Resolve output directory via platform_paths (created by T1-02)
    let output_dir = crate::platform_paths::project_output_dir(&project_id).join(&folder_name);

    fs::create_dir_all(&output_dir).map_err(|e| {
        ApiError::internal(format!("failed to create output dir: {e}"))
    })?;

    let agent = body.agent.unwrap_or_default();
    let metadata = json!({
        "agent": agent,
        "status": "pending",
        "output_type": body.output_type,
        "created_at": now.to_rfc3339(),
        "version": 1,
    });
    let meta_path = output_dir.join("metadata.json");
    fs::write(&meta_path, serde_json::to_string_pretty(&metadata).unwrap_or_default())
        .map_err(|e| ApiError::internal(format!("failed to write metadata.json: {e}")))?;

    let output_path = output_dir.to_string_lossy().to_string();
    let conn = state.get_conn()?;
    conn.execute(
        "INSERT INTO deliverables (task_id, project_id, name, output_type, output_path, \
         status, version) VALUES (?1, ?2, ?3, ?4, ?5, 'pending', 1)",
        rusqlite::params![body.task_id, project_id, name, body.output_type, output_path],
    )
    .map_err(|e| ApiError::internal(format!("create deliverable failed: {e}")))?;

    let id = conn.last_insert_rowid();
    let _ = state.ws_tx.send(json!({
        "type": "deliverable_update",
        "deliverable_id": id,
        "status": "pending",
    }));

    Ok(Json(json!({"id": id, "output_path": output_path})))
}

fn row_to_json(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    Ok(json!({
        "id":          row.get::<_, i64>(0)?,
        "task_id":     row.get::<_, Option<i64>>(1)?,
        "project_id":  row.get::<_, Option<String>>(2)?,
        "name":        row.get::<_, Option<String>>(3)?,
        "output_type": row.get::<_, Option<String>>(4)?,
        "output_path": row.get::<_, Option<String>>(5)?,
        "status":      row.get::<_, Option<String>>(6)?,
        "version":     row.get::<_, Option<i64>>(7)?,
        "approved_by": row.get::<_, Option<String>>(8)?,
        "approved_at": row.get::<_, Option<String>>(9)?,
        "created_at":  row.get::<_, Option<String>>(10)?,
        "updated_at":  row.get::<_, Option<String>>(11)?,
    }))
}
