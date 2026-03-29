//! GET /api/plan-db/execution-context/:plan_id — full CLI execution context
//! POST /api/plan-db/set-worktree/:plan_id — update plan worktree_path

use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use rusqlite::Connection;
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/plan-db/execution-context/:plan_id", get(handle_execution_context))
        .route("/api/plan-db/set-worktree/:plan_id", post(handle_set_worktree))
}

    #[tracing::instrument(skip_all)]
async fn handle_execution_context(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    build_execution_context(&conn, plan_id).map(Json)
}

    #[tracing::instrument(skip_all)]
async fn handle_set_worktree(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let path = body.get("worktree_path").and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing worktree_path"))?;
    let conn = state.get_conn()?;
    set_worktree_in_db(&conn, plan_id, path)?;
    Ok(Json(json!({ "ok": true, "plan_id": plan_id, "worktree_path": path })))
}

/// Core logic: build the full execution context for a plan.
pub fn build_execution_context(conn: &Connection, plan_id: i64) -> Result<Value, ApiError> {
    let plan = query_one(
        conn,
        "SELECT id, name, status, worktree_path, branch_name, \
         description, tasks_total, tasks_done FROM plans WHERE id = ?1",
        rusqlite::params![plan_id],
    )?
    .ok_or_else(|| ApiError::bad_request(format!("plan {plan_id} not found")))?;

    let plan_status = plan["status"].as_str().unwrap_or("unknown");
    let worktree = plan["worktree_path"].as_str().unwrap_or("");
    let branch = plan["branch_name"].as_str().unwrap_or("");
    let plan_name = plan["name"].as_str().unwrap_or("");

    let waves = query_rows(
        conn,
        "SELECT id, wave_id, name, status, tasks_total, tasks_done, position \
         FROM waves WHERE plan_id = ?1 ORDER BY position, id",
        rusqlite::params![plan_id],
    )?;

    let current_wave = waves.iter()
        .find(|w| w["status"].as_str().unwrap_or("") != "done");

    let wave_info = match current_wave {
        Some(w) => {
            let w_id = w["wave_id"].as_str().unwrap_or("");
            let w_status = w["status"].as_str().unwrap_or("pending");
            let w_db_id = w["id"].as_i64().unwrap_or(0);
            let done = w["tasks_done"].as_i64().unwrap_or(0);
            let counts = query_one(
                conn,
                "SELECT \
                 SUM(CASE WHEN status='submitted' THEN 1 ELSE 0 END) AS submitted, \
                 SUM(CASE WHEN status='pending' THEN 1 ELSE 0 END) AS pending, \
                 COUNT(*) AS total FROM tasks WHERE wave_id_fk = ?1",
                rusqlite::params![w_db_id],
            )?.unwrap_or_else(|| json!({}));
            let submitted = counts["submitted"].as_i64().unwrap_or(0);
            let pending = counts["pending"].as_i64().unwrap_or(0);
            let all_sub = pending == 0 && submitted > 0;
            json!({
                "id": w_id, "name": w["name"], "status": w_status,
                "tasks_submitted": submitted,
                "tasks_total": counts["total"].as_i64().unwrap_or(0),
                "tasks_done": done,
                "all_submitted": all_sub,
                "needs_thor": all_sub && w_status != "done",
            })
        }
        None => Value::Null,
    };

    let next_task_row = query_one(
        conn,
        "SELECT t.id, t.task_id, t.title, t.status, t.model, \
         t.executor_agent, t.test_criteria, t.description, \
         w.wave_id AS wave_id_code \
         FROM tasks t JOIN waves w ON t.wave_id_fk = w.id \
         WHERE t.plan_id = ?1 AND t.status = 'pending' \
         ORDER BY w.position, t.id LIMIT 1",
        rusqlite::params![plan_id],
    )?;

    let next_task = match &next_task_row {
        Some(t) => {
            let tc = t["test_criteria"].as_str().unwrap_or("");
            let verify: Vec<&str> = tc.lines().filter(|l| !l.trim().is_empty()).collect();
            json!({
                "db_id": t["id"], "task_id": t["task_id"], "title": t["title"],
                "status": t["status"], "model": t["model"],
                "executor_agent": t["executor_agent"], "verify": verify,
                "wave_id": t["wave_id_code"], "description": t["description"],
            })
        }
        None => Value::Null,
    };

    let decisions: Vec<String> = query_rows(
        conn,
        "SELECT decision FROM decision_log WHERE plan_id = ?1 ORDER BY id LIMIT 10",
        rusqlite::params![plan_id],
    )
    .unwrap_or_default()
    .iter()
    .filter_map(|r| r["decision"].as_str().map(String::from))
    .collect();

    let prompt = build_prompt(
        plan_id, plan_name, worktree, branch, plan_status,
        &wave_info, &next_task, &decisions,
    );

    Ok(json!({
        "ok": true, "plan_id": plan_id, "plan_name": plan_name,
        "status": plan_status, "worktree_path": worktree, "branch": branch,
        "current_wave": wave_info, "next_task": next_task,
        "decisions": decisions, "prompt": prompt,
    }))
}

fn build_prompt(
    plan_id: i64, plan_name: &str, worktree: &str, branch: &str,
    status: &str, wave_info: &Value, next_task: &Value, decisions: &[String],
) -> String {
    if matches!(status, "done" | "completed" | "cancelled") {
        return format!("PLAN {plan_id} ({plan_name}) is {status}. No further action.");
    }
    let needs_thor = wave_info.get("needs_thor").and_then(Value::as_bool).unwrap_or(false);
    if needs_thor {
        let wid = wave_info["id"].as_str().unwrap_or("?");
        return format!(
            "PLAN {plan_id}, WAVE {wid} — all tasks submitted.\n\n\
             Run Thor validation:\n  cvg plan validate {plan_id}\n\n\
             After Thor passes, merge the wave:\n  cvg wave merge {plan_id} {wid}"
        );
    }
    if next_task.is_null() {
        return format!("PLAN {plan_id} ({plan_name}) has no pending tasks.");
    }
    let tid = next_task["task_id"].as_str().unwrap_or("?");
    let db_id = next_task["db_id"].as_i64().unwrap_or(0);
    let title = next_task["title"].as_str().unwrap_or("");
    let wid = next_task["wave_id"].as_str().unwrap_or("?");
    let verify: Vec<&str> = next_task["verify"].as_array()
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let vblock = if verify.is_empty() {
        "(none specified)".to_string()
    } else {
        verify.iter().map(|v| format!("- {v}")).collect::<Vec<_>>().join("\n")
    };
    let dblock = if decisions.is_empty() { String::new() } else {
        format!("\nDECISIONS:\n{}\n",
            decisions.iter().map(|d| format!("- {d}")).collect::<Vec<_>>().join("\n"))
    };
    format!(
        "PLAN {plan_id}, TASK {tid} (DB id: {db_id}), WAVE {wid}\n\n\
         WORKING DIR: {worktree}\nBRANCH: {branch}\n\n\
         DO:\n{title}\n\nVERIFY:\n{vblock}\n{dblock}\n\
         RULES (NON-NEGOTIABLE):\n\
         - ACT IMMEDIATELY. Do NOT ask questions. Do NOT propose alternatives.\n\
         - Max 250 lines per file\n\
         - TDD: write tests first\n\
         - Commit with conventional commits\n\
         - After work: cvg task update {db_id} submitted --summary \"<what you did>\"\n\
         - Do NOT set status to done — only submitted. Thor promotes to done.\n\
         - If task is already done in code (commit exists), submit with summary explaining."
    )
}

/// Update the worktree_path for a plan.
pub fn set_worktree_in_db(conn: &Connection, plan_id: i64, path: &str) -> Result<(), ApiError> {
    let changed = conn.execute(
        "UPDATE plans SET worktree_path = ?1 WHERE id = ?2",
        rusqlite::params![path, plan_id],
    ).map_err(|e| ApiError::internal(format!("update failed: {e}")))?;
    if changed == 0 {
        return Err(ApiError::bad_request(format!("plan {plan_id} not found")));
    }
    Ok(())
}
