// plans_detail: tasks, projects, notifications, plan status update handlers
use super::super::state::{query_rows, ApiError, ServerState};
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

pub async fn api_tasks_distribution(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    Ok(Json(Value::Array(query_rows(
        &conn,
        "SELECT status, COUNT(*) AS count FROM tasks GROUP BY status ORDER BY count DESC",
        [],
    )?)))
}

pub async fn api_tasks_blocked(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    Ok(Json(Value::Array(query_rows(
        &conn,
        "SELECT task_id,title,status,plan_id FROM tasks WHERE status='blocked'",
        [],
    )?)))
}

pub async fn api_plans_assignable(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    Ok(Json(Value::Array(query_rows(
        &conn,
        "SELECT id,name,status,tasks_done,tasks_total,execution_host,human_summary FROM plans WHERE status IN ('todo','doing') ORDER BY id",
        [],
    )?)))
}

pub async fn api_notifications(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    Ok(Json(Value::Array(query_rows(
        &conn,
        "SELECT id,type,title,message,link,link_type,is_read,created_at FROM notifications WHERE is_read=0 ORDER BY created_at DESC LIMIT 20",
        [],
    )?)))
}

pub async fn api_projects(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    Ok(Json(Value::Array(query_rows(
        &conn,
        "SELECT id,name,path FROM projects ORDER BY name COLLATE NOCASE",
        [],
    )?)))
}

#[derive(Deserialize)]
pub struct ProjectCreateBody {
    name: String,
    description: Option<String>,
    repo: Option<String>,
    path: Option<String>,
}

pub async fn api_project_create(
    State(state): State<ServerState>,
    Json(body): Json<ProjectCreateBody>,
) -> Result<Json<Value>, ApiError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("name is required"));
    }
    let path = body
        .path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            body.repo
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        });
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let project_id = format!(
        "{}-{ts}",
        name.to_lowercase()
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
            .collect::<String>()
            .trim_matches('-')
    );

    let conn = state.get_conn()?;
    conn.execute(
        "INSERT INTO projects(id,name,path) VALUES(?1,?2,?3)",
        rusqlite::params![project_id, name, path],
    )
    .map_err(|err| ApiError::internal(format!("create project failed: {err}")))?;

    Ok(Json(json!({
        "ok": true,
        "project": {
            "id": project_id,
            "name": name,
            "path": path,
            "description": body.description.unwrap_or_default(),
            "repo": body.repo.unwrap_or_default()
        }
    })))
}

#[derive(Deserialize)]
pub struct PlanStatusPayload {
    plan_id: i64,
    status: String,
}

pub async fn api_plan_status(
    State(state): State<ServerState>,
    axum::Json(payload): axum::Json<PlanStatusPayload>,
) -> Result<Json<Value>, ApiError> {
    if !matches!(
        payload.status.as_str(),
        "todo" | "doing" | "done" | "cancelled"
    ) {
        return Err(ApiError::bad_request("Invalid status"));
    }
    let conn = state.get_conn()?;
    conn.execute(
        "UPDATE plans SET status=?1 WHERE id=?2",
        rusqlite::params![payload.status, payload.plan_id],
    )
    .map_err(|err| ApiError::internal(format!("status update failed: {err}")))?;
    Ok(Json(json!({"ok": true, "plan_id": payload.plan_id, "status": payload.status})))
}
