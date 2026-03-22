// api_deliverables_handlers: approve and version handlers for deliverables
use super::api_deliverables::get_deliverable;
use super::state::{query_one, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;

#[derive(Deserialize)]
pub struct ApproveBody {
    pub approved_by: String,
}

/// POST /api/deliverables/:id/approve — set status=approved
pub async fn approve_deliverable(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
    Json(body): Json<ApproveBody>,
) -> Result<Json<Value>, ApiError> {
    let approved_by = body.approved_by.trim().to_string();
    if approved_by.is_empty() {
        return Err(ApiError::bad_request("approved_by is required"));
    }

    let now = Utc::now().to_rfc3339();

    // Update DB row
    let rows_changed = {
        let conn = state.get_conn()?;
        conn.execute(
            "UPDATE deliverables SET status='approved', approved_by=?1, \
             approved_at=?2, updated_at=?2 WHERE id=?3",
            rusqlite::params![approved_by, now, id],
        )
        .map_err(|e| ApiError::internal(format!("approve deliverable failed: {e}")))?
    };

    if rows_changed == 0 {
        return Err(ApiError::bad_request(format!("deliverable {id} not found")));
    }

    // Update metadata.json on disk
    update_metadata_status(state.clone(), id, "approved", Some(&approved_by), Some(&now))?;

    let _ = state.ws_tx.send(json!({
        "type": "deliverable_update",
        "deliverable_id": id,
        "status": "approved",
    }));

    get_deliverable(State(state), Path(id)).await
}

/// POST /api/deliverables/:id/version — create new version folder
pub async fn version_deliverable(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let row = query_one(
        &conn,
        "SELECT output_path, version, name, project_id, task_id, output_type \
         FROM deliverables WHERE id=?1",
        rusqlite::params![id],
    )?;
    let row = row.ok_or_else(|| ApiError::bad_request(format!("deliverable {id} not found")))?;

    let current_path = row["output_path"]
        .as_str()
        .ok_or_else(|| ApiError::internal("missing output_path"))?;
    let current_version = row["version"].as_i64().unwrap_or(1);
    let new_version = current_version + 1;

    // Derive new folder path: replace _vN suffix with _v(N+1)
    let new_path = if let Some(base) = current_path.rsplit_once("_v") {
        // Strip the version number part; keep the prefix
        format!("{}_v{new_version}", base.0)
    } else {
        format!("{current_path}_v{new_version}")
    };

    fs::create_dir_all(&new_path)
        .map_err(|e| ApiError::internal(format!("failed to create version dir: {e}")))?;

    // Write new metadata.json in the new version folder
    let now = Utc::now();
    let metadata = json!({
        "agent": "",
        "status": "pending",
        "output_type": row["output_type"],
        "created_at": now.to_rfc3339(),
        "version": new_version,
    });
    let meta_path = std::path::Path::new(&new_path).join("metadata.json");
    fs::write(&meta_path, serde_json::to_string_pretty(&metadata).unwrap_or_default())
        .map_err(|e| ApiError::internal(format!("failed to write metadata.json: {e}")))?;

    // Insert new DB row for the new version
    let name = row["name"].as_str().unwrap_or("");
    let project_id = row["project_id"].as_str().unwrap_or("");
    let task_id = row["task_id"].as_i64();
    let output_type = row["output_type"].as_str().unwrap_or("");

    conn.execute(
        "INSERT INTO deliverables (task_id, project_id, name, output_type, output_path, \
         status, version) VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)",
        rusqlite::params![task_id, project_id, name, output_type, new_path, new_version],
    )
    .map_err(|e| ApiError::internal(format!("create version failed: {e}")))?;

    let new_id = conn.last_insert_rowid();

    let _ = state.ws_tx.send(json!({
        "type": "deliverable_update",
        "deliverable_id": new_id,
        "status": "pending",
    }));

    Ok(Json(json!({
        "id": new_id,
        "version": new_version,
        "output_path": new_path,
        "previous_id": id,
    })))
}

/// Update metadata.json on disk for a given deliverable
fn update_metadata_status(
    state: ServerState,
    id: i64,
    status: &str,
    approved_by: Option<&str>,
    approved_at: Option<&str>,
) -> Result<(), ApiError> {
    let conn = state.get_conn()?;
    let row = query_one(
        &conn,
        "SELECT output_path FROM deliverables WHERE id=?1",
        rusqlite::params![id],
    )?;
    let Some(row) = row else { return Ok(()) };
    let Some(path) = row["output_path"].as_str() else {
        return Ok(());
    };

    let meta_path = std::path::Path::new(path).join("metadata.json");
    if !meta_path.is_file() {
        return Ok(());
    }
    let raw = fs::read_to_string(&meta_path)
        .map_err(|e| ApiError::internal(format!("read metadata: {e}")))?;
    let mut meta: Value = serde_json::from_str(&raw)
        .map_err(|e| ApiError::internal(format!("parse metadata: {e}")))?;

    if let Some(obj) = meta.as_object_mut() {
        obj.insert("status".to_string(), json!(status));
        if let Some(by) = approved_by {
            obj.insert("approved_by".to_string(), json!(by));
        }
        if let Some(at) = approved_at {
            obj.insert("approved_at".to_string(), json!(at));
        }
    }

    fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap_or_default())
        .map_err(|e| ApiError::internal(format!("write metadata: {e}")))?;

    Ok(())
}
