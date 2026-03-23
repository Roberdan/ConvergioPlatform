// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// HTTP API handlers for OpenClaw bridge.
// GET  /api/openclaw/agents — lists agent catalog entries
// POST /api/openclaw/invoke — dispatches a skill request to an agent

use super::state::{query_rows, ApiError, ServerState};
use crate::ipc::skills::executor::create_skill_request;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/openclaw/agents", get(list_agents))
        .route("/api/openclaw/invoke", post(invoke_agent))
}

async fn list_agents(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let agents = query_rows(
        &conn,
        "SELECT name, category, description, model, tools \
         FROM agent_catalog ORDER BY name",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "agents": agents })))
}

#[derive(Debug, Deserialize)]
struct InvokeRequest {
    agent_id: Option<String>,
    message: String,
}

async fn invoke_agent(
    State(state): State<ServerState>,
    Json(req): Json<InvokeRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.message.trim().is_empty() {
        return Err(ApiError::bad_request("message must not be empty"));
    }

    let agent_id = match &req.agent_id {
        Some(id) if !id.trim().is_empty() => id.clone(),
        _ => "ali-orchestrator".to_string(),
    };

    let conn = state.get_conn()?;
    let request_id = create_skill_request(&conn, &agent_id, &req.message)
        .map_err(|e| ApiError::internal(format!("skill request failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "request_id": request_id,
        "agent": agent_id,
        "status": "pending",
    })))
}
