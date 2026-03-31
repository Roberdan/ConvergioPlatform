use super::api_workspace_support::{bind_workspace_context, make_manager};
use super::state::{ApiError, ServerState};
use crate::workspace::deliverables::list_workspace_deliverables;
use crate::workspace::events::EventLogger;
use crate::workspace::git_connector::GitHubConnector;
use crate::workspace::quality_gate::QualityGate;
use crate::workspace::release_agent::ReleaseAgent;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/workspace/list", get(list_workspaces))
        .route("/api/workspace/create", post(create_workspace))
        .route("/api/workspace/delete", post(delete_workspace))
        .route(
            "/api/workspace/status/:workspace_id",
            get(get_workspace_status),
        )
        .route("/api/workspace/events", get(get_workspace_events))
        .route("/api/workspace/quality-gate", post(run_quality_gate))
        .route("/api/workspace/release", post(run_release))
        .route(
            "/api/workspace/deliverables",
            get(get_workspace_deliverables),
        )
}

async fn list_workspaces(
    State(state): State<ServerState>,
    Query(qs): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let plan_id: Option<i64> = qs.get("plan_id").and_then(|v| v.parse().ok()); // intentional: malformed plan_id means "list all workspaces"
    let mgr = make_manager(&state, None, None)?;
    let workspaces = mgr
        .list_workspaces(plan_id)
        .map_err(|e| ApiError::internal(format!("list workspaces failed: {e}")))?;
    Ok(Json(json!({"ok": true, "workspaces": workspaces})))
}

#[derive(Deserialize)]
struct CreateBody {
    #[serde(default)]
    plan_id: Option<i64>,
    #[serde(default)]
    wave_db_id: Option<i64>,
}

async fn create_workspace(
    State(state): State<ServerState>,
    Json(body): Json<CreateBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let mgr = make_manager(&state, body.plan_id, body.wave_db_id)?;
    let ws = mgr
        .create_workspace(body.plan_id, body.wave_db_id)
        .map_err(|e| ApiError::internal(format!("create workspace failed: {e}")))?;
    let conn = state.get_conn()?;
    bind_workspace_context(&conn, &ws)?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"ok": true, "workspace": ws})),
    ))
}

#[derive(Deserialize)]
struct DeleteBody {
    workspace_id: String,
}

async fn delete_workspace(
    State(state): State<ServerState>,
    Json(body): Json<DeleteBody>,
) -> Result<Json<Value>, ApiError> {
    if body.workspace_id.trim().is_empty() {
        return Err(ApiError::bad_request("workspace_id is required"));
    }
    let mgr = make_manager(&state, None, None)?;
    mgr.delete_workspace(&body.workspace_id)
        .map_err(|e| ApiError::internal(format!("delete workspace failed: {e}")))?;
    Ok(Json(json!({"ok": true, "workspace_id": body.workspace_id})))
}

async fn get_workspace_status(
    State(state): State<ServerState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let mgr = make_manager(&state, None, None)?;
    let ws = mgr
        .get_workspace(&workspace_id)
        .map_err(|e| ApiError::internal(format!("get workspace failed: {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("workspace {workspace_id} not found")))?;

    let logger = EventLogger::new(state.pool());
    let recent_events = logger
        .query_events(&workspace_id, Some(10), None)
        .map_err(|e| ApiError::internal(format!("query events failed: {e}")))?;
    Ok(Json(
        json!({"workspace": ws, "recent_events": recent_events}),
    ))
}

async fn get_workspace_events(
    State(state): State<ServerState>,
    Query(qs): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let workspace_id = qs
        .get("workspace_id")
        .filter(|v| !v.is_empty())
        .ok_or_else(|| ApiError::bad_request("workspace_id query param is required"))?
        .clone();
    let limit: Option<i64> = qs.get("limit").and_then(|v| v.parse().ok()); // intentional: invalid limit is ignored so the event log uses its default window
    let since = qs.get("since").map(String::as_str).map(str::to_owned);

    let logger = EventLogger::new(state.pool());
    let events = logger
        .query_events(&workspace_id, limit, since.as_deref())
        .map_err(|e| ApiError::internal(format!("query events failed: {e}")))?;
    Ok(Json(json!({"ok": true, "events": events})))
}

#[derive(Deserialize)]
struct QualityGateBody {
    workspace_id: String,
}

async fn run_quality_gate(
    State(state): State<ServerState>,
    Json(body): Json<QualityGateBody>,
) -> Result<Json<Value>, ApiError> {
    if body.workspace_id.trim().is_empty() {
        return Err(ApiError::bad_request("workspace_id is required"));
    }
    let mgr = make_manager(&state, None, None)?;
    let ws = mgr
        .get_workspace(&body.workspace_id)
        .map_err(|e| ApiError::internal(format!("get workspace failed: {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("workspace {} not found", body.workspace_id)))?;
    let workspace_path = PathBuf::from(&ws.path);
    let gates = QualityGate::run_all(&workspace_path);
    let all_passed = gates.iter().all(|g| g.passed);
    Ok(Json(
        json!({"ok": true, "gates": gates, "all_passed": all_passed}),
    ))
}

async fn get_workspace_deliverables(
    State(state): State<ServerState>,
    Query(qs): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let workspace_id = qs
        .get("workspace_id")
        .filter(|v| !v.is_empty())
        .ok_or_else(|| ApiError::bad_request("workspace_id query param is required"))?
        .clone();
    let pool = state.pool();
    let deliverables = list_workspace_deliverables(&workspace_id, &pool)
        .map_err(|e| ApiError::internal(format!("list deliverables failed: {e}")))?;
    Ok(Json(
        json!({"ok": true, "workspace_id": workspace_id, "deliverables": deliverables}),
    ))
}

#[derive(Deserialize)]
struct ReleaseBody {
    workspace_id: String,
    repo: String,
}

async fn run_release(
    State(state): State<ServerState>,
    Json(body): Json<ReleaseBody>,
) -> Result<Json<Value>, ApiError> {
    if body.workspace_id.trim().is_empty() {
        return Err(ApiError::bad_request("workspace_id is required"));
    }
    if body.repo.trim().is_empty() {
        return Err(ApiError::bad_request("repo is required (e.g. owner/repo)"));
    }
    let token = env::var("GITHUB_TOKEN").unwrap_or_default();
    let connector = Box::new(GitHubConnector {
        github_token: token,
    });
    let logger = EventLogger::new(state.pool());
    let agent = ReleaseAgent::new(connector, logger, state.pool());
    let result = agent
        .release(&body.workspace_id, &body.repo)
        .await
        .map_err(|e| ApiError::internal(format!("release failed: {e}")))?;
    Ok(Json(
        serde_json::to_value(&result).unwrap_or(json!({"ok": true})),
    ))
}
