// Copyright (c) 2026 Roberto D'Angelo
//! Plan review endpoints — register, check, and reset plan_reviews rows.
use super::state::{query_one, ApiError, ServerState};
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/plan-db/review/register", post(handle_review_register))
        .route("/api/plan-db/review/check", get(handle_review_check))
        .route("/api/plan-db/review/reset", post(handle_review_reset))
        .route(
            "/api/plan-db/review/link-by-spec",
            post(handle_review_link_by_spec),
        )
}

/// Valid verdicts accepted by the register endpoint.
const VALID_VERDICTS: &[&str] = &["proceed", "revise", "reject"];

/// POST /api/plan-db/review/register
///
/// Supports two modes:
///   1. Linked to a plan:  body must contain `plan_id` (integer)
///   2. Pre-plan by spec:  body must contain `spec_file` (string path)
///
/// Body: {plan_id?, spec_file?, reviewer_agent, verdict, suggestions?, raw_report?}
async fn handle_review_register(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let plan_id = body.get("plan_id").and_then(Value::as_i64);
    let spec_file = body.get("spec_file").and_then(Value::as_str);

    // Require at least one anchor
    if plan_id.is_none() && spec_file.is_none() {
        return Err(ApiError::bad_request(
            "supply either plan_id or spec_file",
        ));
    }

    let reviewer_agent = body
        .get("reviewer_agent")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing reviewer_agent"))?;
    let verdict = body
        .get("verdict")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing verdict"))?;

    // Validate verdict server-side as well (CLI also validates, but defence-in-depth)
    if !VALID_VERDICTS.contains(&verdict) {
        return Err(ApiError::bad_request(format!(
            "invalid verdict '{verdict}' — must be one of: proceed | revise | reject"
        )));
    }

    let suggestions = body.get("suggestions").and_then(Value::as_str);
    let raw_report = body.get("raw_report").and_then(Value::as_str);

    let conn = state.get_conn()?;

    conn.execute(
        "INSERT INTO plan_reviews \
         (plan_id, spec_file, reviewer_agent, verdict, suggestions, raw_report, reviewed_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
        rusqlite::params![
            plan_id,
            spec_file,
            reviewer_agent,
            verdict,
            suggestions,
            raw_report
        ],
    )
    .map_err(|e| ApiError::internal(format!("review register failed: {e}")))?;

    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .unwrap_or(0);

    Ok(Json(json!({
        "ok": true,
        "id": id,
        "plan_id": plan_id,
        "spec_file": spec_file,
        "reviewer_agent": reviewer_agent,
        "verdict": verdict,
    })))
}

#[derive(Deserialize)]
struct ReviewCheckQuery {
    plan_id: i64,
}

/// GET /api/plan-db/review/check?plan_id=N — count reviews by reviewer type
async fn handle_review_check(
    State(state): State<ServerState>,
    Query(params): Query<ReviewCheckQuery>,
) -> Result<Json<Value>, ApiError> {
    let plan_id = params.plan_id;
    let conn = state.get_conn()?;
    let conn = &conn;

    let reviewer_count = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM plan_reviews \
         WHERE plan_id = ?1 AND reviewer_agent LIKE '%reviewer%' \
         AND reviewer_agent NOT LIKE '%business%'",
        rusqlite::params![plan_id],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    let business_count = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM plan_reviews \
         WHERE plan_id = ?1 AND (reviewer_agent LIKE '%business%' \
           OR reviewer_agent LIKE '%advisor%')",
        rusqlite::params![plan_id],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    let challenger_count = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM plan_reviews \
         WHERE plan_id = ?1 AND reviewer_agent LIKE '%challenger%'",
        rusqlite::params![plan_id],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    let user_approved = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM plan_reviews \
         WHERE plan_id = ?1 AND reviewer_agent = 'user-approval'",
        rusqlite::params![plan_id],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    let total = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM plan_reviews WHERE plan_id = ?1",
        rusqlite::params![plan_id],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "total": total,
        "reviewer": reviewer_count,
        "business": business_count,
        "challenger": challenger_count,
        "user_approved": user_approved,
    })))
}

/// POST /api/plan-db/review/reset — delete all reviews for a plan
/// Body: {plan_id}
async fn handle_review_reset(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let plan_id = body
        .get("plan_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing plan_id"))?;

    let conn = state.get_conn()?;

    let deleted = conn
        .execute(
            "DELETE FROM plan_reviews WHERE plan_id = ?1",
            rusqlite::params![plan_id],
        )
        .map_err(|e| ApiError::internal(format!("review reset failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "deleted": deleted,
    })))
}

/// POST /api/plan-db/review/link-by-spec
///
/// Called automatically by `cvg plan create` after a plan is created from a spec file.
/// Links all unlinked plan_reviews that match `spec_file` to the new `plan_id`.
///
/// Body: { plan_id: i64, spec_file: String }
async fn handle_review_link_by_spec(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let plan_id = body
        .get("plan_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing plan_id"))?;
    let spec_file = body
        .get("spec_file")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing spec_file"))?;

    let conn = state.get_conn()?;

    let linked = conn
        .execute(
            "UPDATE plan_reviews SET plan_id = ?1 \
             WHERE spec_file = ?2 AND plan_id IS NULL",
            rusqlite::params![plan_id, spec_file],
        )
        .map_err(|e| ApiError::internal(format!("review link failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "spec_file": spec_file,
        "linked": linked,
    })))
}

#[cfg(test)]
#[path = "api_plan_db_review_tests.rs"]
mod tests;
