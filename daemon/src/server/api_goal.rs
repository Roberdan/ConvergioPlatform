// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// F-T4-03: Goal decomposer API.
// POST /api/goal/decompose — natural-language goal to multi-domain wave plan.
// POST /api/goal/execute   — stub: queue plan creation from a decomposed goal.

use super::state::{ApiError, ServerState};
use crate::orchestrator::goal_decomposer;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/goal/decompose", post(handle_decompose))
        .route("/api/goal/execute", post(handle_execute))
}

/// POST /api/goal/decompose
/// Body: `{"goal": "Launch recipe SaaS"}`
/// Returns: `{ok, goal_id, domains, agents, waves, estimated_tasks}`
#[tracing::instrument(skip_all)]
pub async fn handle_decompose(
    State(_state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let goal = body
        .get("goal")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing goal"))?;
    if goal.trim().is_empty() {
        return Err(ApiError::bad_request("goal must not be empty"));
    }
    let plan = goal_decomposer::decompose(goal);
    Ok(Json(json!({
        "ok": true,
        "goal_id": plan.goal_id,
        "domains": plan.domains,
        "agents": plan.agents,
        "waves": plan.waves,
        "estimated_tasks": plan.estimated_tasks,
    })))
}

/// POST /api/goal/execute
/// Body: `{"goal_id": "goal-launch-recipe-saas"}`
/// Stub: returns a queued plan_id. Actual LLM orchestration is deferred to @planner.
#[tracing::instrument(skip_all)]
pub async fn handle_execute(
    State(_state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let goal_id = body
        .get("goal_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing goal_id"))?;
    if goal_id.trim().is_empty() {
        return Err(ApiError::bad_request("goal_id must not be empty"));
    }
    // Deterministic plan_id from goal_id; real impl would call @planner and insert into plan_db.
    let plan_id = format!("plan-{}", goal_id.trim_start_matches("goal-"));
    Ok(Json(json!({
        "ok": true,
        "goal_id": goal_id,
        "plan_id": plan_id,
        "status": "queued",
        "message": "Plan creation queued. Track via /api/plan-db/list.",
    })))
}
