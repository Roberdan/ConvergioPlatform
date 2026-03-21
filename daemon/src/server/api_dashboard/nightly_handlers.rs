// nightly_handlers: nightly job config, create, toggle, events and coordinator
use super::super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

pub async fn api_nightly_config_get(
    State(state): State<ServerState>,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT id, name, description, schedule, script_path, target_host, enabled, run_fixes, timeout_sec FROM nightly_job_definitions WHERE project_id=?1 ORDER BY name",
        rusqlite::params![&project_id],
    )?;
    Ok(Json(json!({"ok": true, "project_id": project_id, "definitions": rows})))
}

#[derive(Deserialize)]
pub struct ConfigUpdate {
    run_fixes: Option<i32>,
    schedule: Option<String>,
    timeout_sec: Option<i32>,
}

pub async fn api_nightly_config_update(
    State(state): State<ServerState>,
    Path(project_id): Path<String>,
    axum::Json(payload): axum::Json<ConfigUpdate>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let mut updated_fields = 0usize;

    if let Some(run_fixes) = payload.run_fixes {
        updated_fields += conn
            .execute(
                "UPDATE nightly_job_definitions SET run_fixes=?1 WHERE project_id=?2",
                rusqlite::params![run_fixes, &project_id],
            )
            .map_err(|err| ApiError::internal(format!("config update failed: {err}")))?;
    }
    if let Some(schedule) = payload.schedule {
        let schedule = schedule.trim().to_string();
        if schedule.is_empty() {
            return Err(ApiError::bad_request("schedule must not be empty"));
        }
        updated_fields += conn
            .execute(
                "UPDATE nightly_job_definitions SET schedule=?1 WHERE project_id=?2",
                rusqlite::params![schedule, &project_id],
            )
            .map_err(|err| ApiError::internal(format!("config update failed: {err}")))?;
    }
    if let Some(timeout_sec) = payload.timeout_sec {
        updated_fields += conn
            .execute(
                "UPDATE nightly_job_definitions SET timeout_sec=?1 WHERE project_id=?2",
                rusqlite::params![timeout_sec, &project_id],
            )
            .map_err(|err| ApiError::internal(format!("config update failed: {err}")))?;
    }

    Ok(Json(json!({"ok": true, "updated": project_id, "rows_affected": updated_fields})))
}

pub async fn api_nightly_def_toggle(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let updated = conn
        .execute(
            "UPDATE nightly_job_definitions SET enabled = CASE WHEN enabled=1 THEN 0 ELSE 1 END WHERE id=?1",
            rusqlite::params![id],
        )
        .map_err(|err| ApiError::internal(format!("toggle failed: {err}")))?;
    if updated == 0 {
        return Err(ApiError::bad_request(format!(
            "nightly job definition {id} not found"
        )));
    }
    let enabled: i64 = conn
        .query_row(
            "SELECT enabled FROM nightly_job_definitions WHERE id=?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .map_err(|err| ApiError::internal(format!("toggle readback failed: {err}")))?;
    Ok(Json(json!({"ok": true, "id": id, "enabled": enabled == 1})))
}

#[derive(Deserialize)]
pub struct NightlyJobCreatePayload {
    pub name: String,
    pub script_path: String,
    #[serde(default = "default_schedule")]
    pub schedule: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_host")]
    pub target_host: String,
    #[serde(default = "default_project")]
    pub project_id: String,
}
fn default_schedule() -> String {
    "0 3 * * *".to_string()
}
fn default_host() -> String {
    "local".to_string()
}
fn default_project() -> String {
    "mirrorbuddy".to_string()
}

pub async fn api_nightly_job_create(
    State(state): State<ServerState>,
    axum::Json(payload): axum::Json<NightlyJobCreatePayload>,
) -> Result<Json<Value>, ApiError> {
    let name = payload.name.trim().to_string();
    let script = payload.script_path.trim().to_string();
    if name.is_empty() || script.is_empty() {
        return Err(ApiError::bad_request("name and script_path are required"));
    }
    let conn = state.get_conn()?;
    conn.execute(
        "INSERT INTO nightly_job_definitions (name,description,schedule,script_path,target_host,project_id) VALUES (?1,?2,?3,?4,?5,?6)",
        rusqlite::params![name, payload.description, payload.schedule, script, payload.target_host, payload.project_id],
    )
    .map_err(|err| ApiError::internal(format!("create job failed: {err}")))?;
    Ok(Json(json!({"ok": true, "name": name})))
}

pub async fn api_events(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    Ok(Json(Value::Array(query_rows(
        &conn,
        "SELECT id,event_type,plan_id,source_peer,payload,status,created_at FROM mesh_events ORDER BY created_at DESC LIMIT 50",
        [],
    )?)))
}

pub async fn api_coordinator_status(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let pending = query_one(
        &conn,
        "SELECT COUNT(*) AS pending_events FROM mesh_events WHERE status='pending'",
        [],
    )?
    .unwrap_or_else(|| json!({"pending_events": 0}));
    Ok(Json(json!({"running": false, "pid": "", "pending_events": pending["pending_events"]})))
}

pub async fn api_coordinator_toggle() -> Json<Value> {
    Json(json!({"ok": true, "action": "noop"}))
}
