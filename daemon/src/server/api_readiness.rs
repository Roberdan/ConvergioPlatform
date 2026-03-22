// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
//! Readiness check endpoint — validates plan gates before execution.
use super::state::{ApiError, ServerState};
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new().route(
        "/api/plan-db/readiness/:plan_id",
        get(handle_readiness),
    )
}

#[derive(Debug, Serialize)]
pub struct Gate {
    pub name: String,
    pub passed: bool,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct ReadinessResult {
    pub ready: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub gates: Vec<Gate>,
}

/// Check all readiness gates for a plan. Returns an error if plan not found.
pub fn check_readiness(
    conn: &Connection,
    plan_id: i64,
) -> Result<ReadinessResult, ApiError> {
    // Verify plan exists
    let tasks_total: i64 = conn
        .query_row(
            "SELECT tasks_total FROM plans WHERE id = ?1",
            rusqlite::params![plan_id],
            |r| r.get(0),
        )
        .map_err(|_| ApiError::bad_request(format!("plan {plan_id} not found")))?;

    let mut errors = Vec::new();
    let warnings: Vec<String> = Vec::new();
    let mut gates = Vec::new();

    // Gate 1: review_approved — at least one review exists
    let review_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM plan_reviews WHERE plan_id = ?1",
            rusqlite::params![plan_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let review_passed = review_count > 0;
    if !review_passed {
        errors.push("No review registered for this plan".to_string());
    }
    gates.push(Gate {
        name: "review_approved".to_string(),
        passed: review_passed,
        reason: if review_passed {
            format!("{review_count} review(s) found")
        } else {
            "No review registered".to_string()
        },
    });

    // Gate 2: spec_imported — plan has tasks (tasks_total > 0)
    let spec_passed = tasks_total > 0;
    if !spec_passed {
        errors.push("No tasks imported (tasks_total = 0)".to_string());
    }
    gates.push(Gate {
        name: "spec_imported".to_string(),
        passed: spec_passed,
        reason: if spec_passed {
            format!("{tasks_total} task(s) imported")
        } else {
            "No tasks imported".to_string()
        },
    });

    // Gate 3: all_tasks_have_model — every task has a non-empty model
    let missing_model: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks \
             WHERE plan_id = ?1 AND (model IS NULL OR model = '')",
            rusqlite::params![plan_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let model_passed = missing_model == 0 && tasks_total > 0;
    if !model_passed && tasks_total > 0 {
        errors.push(format!(
            "{missing_model} task(s) missing model assignment"
        ));
    }
    gates.push(Gate {
        name: "all_tasks_have_model".to_string(),
        passed: model_passed,
        reason: if model_passed {
            "All tasks have model assigned".to_string()
        } else if tasks_total == 0 {
            "No tasks to check".to_string()
        } else {
            format!("{missing_model} task(s) missing model")
        },
    });

    let ready = errors.is_empty() && gates.iter().all(|g| g.passed);

    Ok(ReadinessResult {
        ready,
        errors,
        warnings,
        gates,
    })
}

/// GET /api/plan-db/readiness/:plan_id
async fn handle_readiness(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let result = check_readiness(&conn, plan_id)?;

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "ready": result.ready,
        "errors": result.errors,
        "warnings": result.warnings,
        "gates": result.gates,
    })))
}

#[cfg(test)]
#[path = "api_readiness_tests.rs"]
mod tests;
