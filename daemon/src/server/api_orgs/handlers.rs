use crate::server::api_ipc::ensure_ipc_schema;
use super::budget::guard_member_action_budget;
use crate::server::state::{query_one, query_rows, ApiError, ServerState};
use crate::server::ws_brain_org::{broadcast_agent_factory, broadcast_org_topology, broadcast_org_update};
use axum::extract::{Path, State};
use axum::{http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateOrgRequest {
    pub id: Option<String>,
    pub mission: String,
    pub objectives: String,
    pub ceo_agent: String,
    pub budget: f64,
}

#[derive(Deserialize)]
pub struct UpdateOrgRequest {
    pub status: Option<String>,
    pub budget: Option<f64>,
}

#[derive(Deserialize)]
pub struct AddMemberRequest {
    pub agent: String,
    pub role: String,
    pub department: Option<String>,
}

#[derive(Deserialize)]
pub struct RegisterServiceRequest {
    pub name: String,
    pub endpoint: String,
    pub status: Option<String>,
    pub metadata: Option<Value>,
}

pub async fn create_org(
    State(state): State<ServerState>,
    Json(body): Json<CreateOrgRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let id = body
        .id
        .unwrap_or_else(|| format!("org-{}", Uuid::new_v4().simple()));
    conn.execute(
        "INSERT INTO ipc_orgs(id, mission, objectives, ceo_agent, budget, status)
         VALUES (?1, ?2, ?3, ?4, ?5, 'active')",
        rusqlite::params![id, body.mission, body.objectives, body.ceo_agent, body.budget],
    )
    .map_err(|e| ApiError::internal(format!("create org failed: {e}")))?;
    conn.execute(
        "INSERT OR IGNORE INTO ipc_org_members(id, org_id, agent, role, department)
         VALUES (?1, ?2, ?3, 'ceo', 'executive')",
        rusqlite::params![
            format!("member-{}", Uuid::new_v4().simple()),
            id,
            body.ceo_agent
        ],
    )
    .map_err(|e| ApiError::internal(format!("add ceo member failed: {e}")))?;
    conn.execute(
        "INSERT OR IGNORE INTO ipc_channels(name, description, created_by)
         VALUES (?1, ?2, ?3)",
        rusqlite::params![format!("org:{id}"), "Org channel namespace", "system"],
    )
    .map_err(|e| ApiError::internal(format!("create channel failed: {e}")))?;
    broadcast_org_update(&state, &id, "created");
    broadcast_org_topology(&state);
    Ok((StatusCode::CREATED, Json(json!({ "ok": true, "org_id": id }))))
}

pub async fn list_orgs(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let orgs = query_rows(
        &conn,
        "SELECT id, mission, objectives, ceo_agent, budget, status, created_at, updated_at
         FROM ipc_orgs ORDER BY created_at DESC",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "orgs": orgs })))
}

pub async fn get_org(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let org = query_one(
        &conn,
        "SELECT id, mission, objectives, ceo_agent, budget, status, created_at, updated_at
         FROM ipc_orgs WHERE id = ?1",
        rusqlite::params![id],
    )?
    .ok_or_else(|| ApiError::not_found("org not found"))?;
    let members = query_rows(
        &conn,
        "SELECT org_id, agent, role, department, joined_at
         FROM ipc_org_members WHERE org_id = ?1 ORDER BY joined_at DESC",
        rusqlite::params![id],
    )?;
    let services = query_rows(
        &conn,
        "SELECT id, org_id, name, endpoint, status, metadata, registered_at
         FROM ipc_org_services WHERE org_id = ?1 ORDER BY registered_at DESC",
        rusqlite::params![id],
    )?;
    Ok(Json(json!({
        "ok": true,
        "org": org,
        "members": members,
        "services": services
    })))
}

pub async fn update_org(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateOrgRequest>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let changed = conn
        .execute(
            "UPDATE ipc_orgs
             SET status = COALESCE(?2, status),
                 budget = COALESCE(?3, budget),
                 updated_at = (strftime('%Y-%m-%dT%H:%M:%f','now'))
             WHERE id = ?1",
            rusqlite::params![id, body.status, body.budget],
        )
        .map_err(|e| ApiError::internal(format!("update org failed: {e}")))?;
    if changed == 0 {
        return Err(ApiError::not_found("org not found"));
    }
    broadcast_org_update(&state, &id, "updated");
    broadcast_org_topology(&state);
    Ok(Json(json!({ "ok": true, "updated": changed })))
}

pub async fn add_member(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(body): Json<AddMemberRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    guard_member_action_budget(&state, &conn, &id, "add_member")?;
    conn.execute(
        "INSERT INTO ipc_org_members(id, org_id, agent, role, department)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            format!("member-{}", Uuid::new_v4().simple()),
            id,
            body.agent,
            body.role,
            body.department
        ],
    )
    .map_err(|e| ApiError::internal(format!("add member failed: {e}")))?;
    broadcast_agent_factory(&state, &id, &body.agent, &body.role);
    broadcast_org_topology(&state);
    Ok((StatusCode::CREATED, Json(json!({ "ok": true }))))
}

pub async fn remove_member(
    State(state): State<ServerState>,
    Path((id, agent)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    guard_member_action_budget(&state, &conn, &id, "remove_member")?;
    let deleted = conn
        .execute(
            "DELETE FROM ipc_org_members WHERE org_id = ?1 AND agent = ?2",
            rusqlite::params![id, agent],
        )
        .map_err(|e| ApiError::internal(format!("remove member failed: {e}")))?;
    Ok(Json(json!({ "ok": true, "deleted": deleted })))
}

pub async fn register_service(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(body): Json<RegisterServiceRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    conn.execute(
        "INSERT INTO ipc_org_services(id, org_id, name, endpoint, status, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            format!("svc-{}", Uuid::new_v4().simple()),
            id,
            body.name,
            body.endpoint,
            body.status.unwrap_or_else(|| "active".to_string()),
            body.metadata.map(|m| m.to_string())
        ],
    )
    .map_err(|e| ApiError::internal(format!("register service failed: {e}")))?;
    Ok((StatusCode::CREATED, Json(json!({ "ok": true }))))
}

pub async fn list_services(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let services = query_rows(
        &conn,
        "SELECT id, org_id, name, endpoint, status, metadata, registered_at
         FROM ipc_org_services WHERE org_id = ?1 ORDER BY registered_at DESC",
        rusqlite::params![id],
    )?;
    Ok(Json(json!({ "ok": true, "services": services })))
}
