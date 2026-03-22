use super::api_plan_db_import_defaults::apply_defaults;
use super::api_plan_db_import_parsers::parse_waves;
use super::plan_lifecycle_guards;
use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/plan-db/import", post(handle_import))
}

/// POST /api/plan-db/import — bulk import waves+tasks from JSON/YAML spec
async fn handle_import(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let plan_id = body
        .get("plan_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing plan_id"))?;

    // Parse waves from spec (support raw YAML string or JSON object)
    let mut waves = parse_waves(&body)?;

    let conn = state.get_conn()?;
    let conn = &conn;

    // Guard: plan must exist and be in importable state (draft/todo/approved)
    plan_lifecycle_guards::require_plan_importable(plan_id, conn)
        .map_err(ApiError::conflict)?;

    // Verify plan exists and get project_id
    let project_id: String = conn
        .query_row(
            "SELECT project_id FROM plans WHERE id = ?1",
            rusqlite::params![plan_id],
            |r| r.get(0),
        )
        .map_err(|_| ApiError::bad_request(format!("plan {plan_id} not found")))?;

    let mut waves_created = 0usize;
    let mut tasks_created = 0usize;
    let mut tasks_total = 0i64;

    for (pos, wave) in waves.iter_mut().enumerate() {
        conn.execute(
            "INSERT INTO waves (plan_id, project_id, wave_id, name, status, \
             position, depends_on, estimated_hours) \
             VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7)",
            rusqlite::params![
                plan_id,
                project_id,
                wave.id,
                wave.name,
                pos as i64,
                wave.depends_on,
                wave.estimated_hours,
            ],
        )
        .map_err(|e| ApiError::internal(format!("wave insert failed: {e}")))?;

        let wave_db_id: i64 = conn
            .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
            .map_err(|e| ApiError::internal(format!("rowid failed: {e}")))?;

        waves_created += 1;
        let wave_task_count = wave.tasks.len() as i64;

        for task in &mut wave.tasks {
            // Apply smart defaults before insert (model, validator, verify, effort)
            apply_defaults(task);

            let criteria_json = task
                .test_criteria
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_default();

            // Serialize verify commands as newline-separated string for storage
            let verify_str = if task.verify.is_empty() {
                None
            } else {
                Some(task.verify.join("\n"))
            };

            conn.execute(
                "INSERT INTO tasks (plan_id, project_id, wave_id_fk, wave_id, \
                 task_id, title, status, priority, type, description, \
                 test_criteria, model, assignee, output_type, validator_agent, \
                 effort_level, notes) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?8, ?9, ?10, ?11, ?12, \
                 ?13, ?14, ?15, ?16)",
                rusqlite::params![
                    plan_id,
                    project_id,
                    wave_db_id,
                    wave.id,
                    task.id,
                    task.title,
                    task.priority,
                    task.task_type,
                    task.description,
                    criteria_json,
                    task.model,
                    task.assignee,
                    task.output_type,
                    task.validator_agent,
                    task.effort_level,
                    verify_str,
                ],
            )
            .map_err(|e| ApiError::internal(format!("task insert failed: {e}")))?;

            tasks_created += 1;
        }
        tasks_total += wave_task_count;

        // Update wave task count
        conn.execute(
            "UPDATE waves SET tasks_total = ?1 WHERE id = ?2",
            rusqlite::params![wave_task_count, wave_db_id],
        )
        .map_err(|e| ApiError::internal(format!("wave count update failed: {e}")))?;
    }

    // Update plan task total and waves_total
    conn.execute(
        "UPDATE plans SET tasks_total = tasks_total + ?1, \
         waves_total = COALESCE(waves_total, 0) + ?2, \
         updated_at = datetime('now') \
         WHERE id = ?3",
        rusqlite::params![tasks_total, waves_created as i64, plan_id],
    )
    .map_err(|e| ApiError::internal(format!("plan count update failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "waves_created": waves_created,
        "tasks_created": tasks_created,
    })))
}

#[cfg(test)]
#[path = "api_plan_db_import_tests.rs"]
mod tests;
