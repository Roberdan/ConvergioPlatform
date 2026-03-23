// Workspace REST API — CRUD endpoints for agent workspace management.
// Why: Plan 698 workspace layer; agents need isolated git worktrees tracked in DB.
use super::state::{ApiError, ServerState};
use crate::workspace::core::WorkspaceManager;
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

// GET /api/workspace/list?plan_id=
async fn list_workspaces(
    State(state): State<ServerState>,
    Query(qs): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let plan_id: Option<i64> = qs.get("plan_id").and_then(|v| v.parse().ok());
    let mgr = make_manager(&state);
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

// POST /api/workspace/create
async fn create_workspace(
    State(state): State<ServerState>,
    Json(body): Json<CreateBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let mgr = make_manager(&state);
    let ws = mgr
        .create_workspace(body.plan_id, body.wave_db_id)
        .map_err(|e| ApiError::internal(format!("create workspace failed: {e}")))?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"ok": true, "workspace": ws})),
    ))
}

#[derive(Deserialize)]
struct DeleteBody {
    workspace_id: String,
}

// POST /api/workspace/delete
async fn delete_workspace(
    State(state): State<ServerState>,
    Json(body): Json<DeleteBody>,
) -> Result<Json<Value>, ApiError> {
    if body.workspace_id.trim().is_empty() {
        return Err(ApiError::bad_request("workspace_id is required"));
    }
    let mgr = make_manager(&state);
    mgr.delete_workspace(&body.workspace_id)
        .map_err(|e| ApiError::internal(format!("delete workspace failed: {e}")))?;
    Ok(Json(json!({"ok": true, "workspace_id": body.workspace_id})))
}

// GET /api/workspace/status/:workspace_id
async fn get_workspace_status(
    State(state): State<ServerState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let mgr = make_manager(&state);
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

// GET /api/workspace/events?workspace_id=&limit=&since=
async fn get_workspace_events(
    State(state): State<ServerState>,
    Query(qs): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let workspace_id = qs
        .get("workspace_id")
        .filter(|v| !v.is_empty())
        .ok_or_else(|| ApiError::bad_request("workspace_id query param is required"))?
        .clone();
    let limit: Option<i64> = qs.get("limit").and_then(|v| v.parse().ok());
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

// POST /api/workspace/quality-gate
// Body: {"workspace_id": "<id>"}
// Returns: {"ok": true, "gates": [...], "all_passed": bool}
async fn run_quality_gate(
    State(state): State<ServerState>,
    Json(body): Json<QualityGateBody>,
) -> Result<Json<Value>, ApiError> {
    if body.workspace_id.trim().is_empty() {
        return Err(ApiError::bad_request("workspace_id is required"));
    }
    let mgr = make_manager(&state);
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

// GET /api/workspace/deliverables?workspace_id=
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

// POST /api/workspace/release
// Body: {"workspace_id": "<id>", "repo": "owner/repo"}
// Returns: ReleaseResult JSON — full pipeline: quality gate → commit → push → PR → merge.
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
        .map_err(|e| ApiError::internal(format!("release failed: {e}")))?;
    Ok(Json(
        serde_json::to_value(&result).unwrap_or(json!({"ok": true})),
    ))
}

fn make_manager(state: &ServerState) -> WorkspaceManager {
    let repo_root = repo_root_from_env();
    WorkspaceManager::new(state.pool(), repo_root)
}

fn repo_root_from_env() -> PathBuf {
    env::var("CONVERGIO_REPO_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            env::var("HOME")
                .map(|h| PathBuf::from(h).join("GitHub/ConvergioPlatform"))
                .unwrap_or_else(|_| PathBuf::from("."))
        })
}
