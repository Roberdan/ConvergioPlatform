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
        .route(
            "/api/plan-db/review/register",
            post(handle_review_register),
        )
        .route("/api/plan-db/review/check", get(handle_review_check))
        .route("/api/plan-db/review/reset", post(handle_review_reset))
}

/// POST /api/plan-db/review/register — insert a plan review record
/// Body: {plan_id, reviewer_agent, verdict, suggestions?, raw_report?}
async fn handle_review_register(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let plan_id = body
        .get("plan_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing plan_id"))?;
    let reviewer_agent = body
        .get("reviewer_agent")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing reviewer_agent"))?;
    let verdict = body
        .get("verdict")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing verdict"))?;
    let suggestions = body.get("suggestions").and_then(Value::as_str);
    let raw_report = body.get("raw_report").and_then(Value::as_str);

    let conn = state.get_conn()?;

    conn.execute(
        "INSERT INTO plan_reviews \
         (plan_id, reviewer_agent, verdict, suggestions, raw_report, reviewed_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
        rusqlite::params![plan_id, reviewer_agent, verdict, suggestions, raw_report],
    )
    .map_err(|e| ApiError::internal(format!("review register failed: {e}")))?;

    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .unwrap_or(0);

    Ok(Json(json!({
        "ok": true,
        "id": id,
        "plan_id": plan_id,
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

#[cfg(test)]
mod tests {
    use crate::db::PlanDb;
    use crate::server::state::query_one;

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE plans (
                     id INTEGER PRIMARY KEY, project_id TEXT, name TEXT, status TEXT
                 );
                 CREATE TABLE plan_reviews (
                     id INTEGER PRIMARY KEY, plan_id INTEGER, reviewer_agent TEXT,
                     verdict TEXT, suggestions TEXT, raw_report TEXT,
                     reviewed_at TEXT DEFAULT (datetime('now'))
                 );
                 INSERT INTO plans (id, project_id, name, status)
                     VALUES (1, 'test', 'Test Plan', 'draft');",
            )
            .expect("schema");
        db
    }

    #[test]
    fn review_register_inserts_row() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict) \
             VALUES (1, 'plan-reviewer', 'approved')",
            [],
        )
        .unwrap();

        let row = query_one(
            conn,
            "SELECT COUNT(*) AS c FROM plan_reviews WHERE plan_id = 1",
            [],
        )
        .expect("query")
        .expect("row");

        assert_eq!(row.get("c").and_then(|v| v.as_i64()), Some(1));
    }

    #[test]
    fn review_check_counts_by_type() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute_batch(
            "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict)
             VALUES (1, 'plan-reviewer', 'approved'),
                    (1, 'plan-business-advisor', 'approved'),
                    (1, 'challenger', 'proceed');",
        )
        .unwrap();

        let reviewer: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM plan_reviews \
                 WHERE plan_id = 1 AND reviewer_agent LIKE '%reviewer%' \
                 AND reviewer_agent NOT LIKE '%business%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(reviewer, 1);

        let business: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM plan_reviews \
                 WHERE plan_id = 1 AND (reviewer_agent LIKE '%business%' \
                   OR reviewer_agent LIKE '%advisor%')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(business, 1);
    }

    #[test]
    fn review_reset_deletes_all_for_plan() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute_batch(
            "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict)
             VALUES (1, 'plan-reviewer', 'approved'),
                    (1, 'challenger', 'proceed');",
        )
        .unwrap();

        let deleted = conn
            .execute("DELETE FROM plan_reviews WHERE plan_id = 1", [])
            .unwrap();
        assert_eq!(deleted, 2);

        let remaining: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM plan_reviews WHERE plan_id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 0);
    }
}
