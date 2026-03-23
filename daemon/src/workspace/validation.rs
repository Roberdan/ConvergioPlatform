// Workspace validation — replaces validate-task.sh + validate-wave.sh.
// Enforces sequential wave execution and Thor-only validation authority.
// Why: Plan 698 — centralise validation in daemon, remove bash script drift.

use crate::server::state_init::ConnPool;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateTaskResult {
    pub task_db_id: i64,
    pub old_status: String,
    pub new_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateWaveResult {
    pub wave_db_id: i64,
    pub tasks_validated: i64,
    pub wave_status: String,
}

/// Thor agent identifiers authorised to validate tasks/waves.
const AUTHORISED_VALIDATORS: &[&str] =
    &["thor", "thor-quality-assurance-guardian", "thor-per-wave"];

fn pool_err(e: r2d2::Error) -> String {
    format!("pool error: {e}")
}

/// Validate a single task: submitted→done, or backfill validated_at on done tasks.
/// Only Thor agents may call this. Updates wave/plan tasks_done counters.
pub fn validate_task(
    task_db_id: i64,
    validated_by: &str,
    pool: &ConnPool,
) -> Result<ValidateTaskResult, String> {
    if !AUTHORISED_VALIDATORS.contains(&validated_by) {
        return Err(format!(
            "validator '{validated_by}' is not authorized; must be one of: {}",
            AUTHORISED_VALIDATORS.join(", ")
        ));
    }
    let conn = pool.get().map_err(pool_err)?;
    let (old_status, validated_at, wave_id_fk, plan_id): (
        String,
        Option<String>,
        Option<i64>,
        Option<i64>,
    ) = conn
        .query_row(
            "SELECT status, validated_at, wave_id_fk, plan_id FROM tasks WHERE id = ?1",
            params![task_db_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|e| format!("task {task_db_id} not found: {e}"))?;

    match old_status.as_str() {
        "submitted" => {
            conn.execute(
                "UPDATE tasks SET status='done', validated_at=datetime('now'), validated_by=?1,
                 completed_at=COALESCE(completed_at,datetime('now')) WHERE id=?2",
                params![validated_by, task_db_id],
            )
            .map_err(|e| format!("update task: {e}"))?;
            if let Some(wid) = wave_id_fk {
                conn.execute(
                    "UPDATE waves SET tasks_done=tasks_done+1 WHERE id=?1",
                    params![wid],
                )
                .map_err(|e| format!("wave tasks_done: {e}"))?;
            }
            if let Some(pid) = plan_id {
                conn.execute(
                    "UPDATE plans SET tasks_done=tasks_done+1 WHERE id=?1",
                    params![pid],
                )
                .map_err(|e| format!("plan tasks_done: {e}"))?;
            }
            Ok(ValidateTaskResult {
                task_db_id,
                old_status,
                new_status: "done".into(),
            })
        }
        "done" if validated_at.is_none() => {
            // Backfill stamp for tasks marked done without going through validate_task
            conn.execute(
                "UPDATE tasks SET validated_at=datetime('now'), validated_by=?1 WHERE id=?2",
                params![validated_by, task_db_id],
            )
            .map_err(|e| format!("backfill validated_at: {e}"))?;
            Ok(ValidateTaskResult {
                task_db_id,
                old_status: "done".into(),
                new_status: "done".into(),
            })
        }
        other => Err(format!(
            "task {task_db_id} has status '{other}'; must be 'submitted' or 'done' to validate"
        )),
    }
}

/// Validate an entire wave: batch-promote submitted→done, verify completeness, mark wave done.
pub fn validate_wave(
    wave_db_id: i64,
    validated_by: &str,
    pool: &ConnPool,
) -> Result<ValidateWaveResult, String> {
    if !AUTHORISED_VALIDATORS.contains(&validated_by) {
        return Err(format!(
            "validator '{validated_by}' is not authorized; must be one of: {}",
            AUTHORISED_VALIDATORS.join(", ")
        ));
    }

    let conn = pool.get().map_err(pool_err)?;

    // Check for unresolved tasks (pending/in_progress/blocked) — these block wave validation
    let unresolved: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks
             WHERE wave_id_fk = ?1 AND status IN ('pending', 'in_progress', 'blocked')",
            params![wave_db_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("unresolved check failed: {e}"))?;

    if unresolved > 0 {
        return Err(format!(
            "wave {wave_db_id} has {unresolved} unresolved task(s) (pending/in_progress/blocked); \
             resolve all tasks before validating wave"
        ));
    }

    // Batch-promote submitted→done
    let tasks_validated = conn
        .execute(
            "UPDATE tasks
             SET status = 'done', validated_at = datetime('now'), validated_by = ?1,
                 completed_at = COALESCE(completed_at, datetime('now'))
             WHERE wave_id_fk = ?2 AND status = 'submitted'",
            params![validated_by, wave_db_id],
        )
        .map_err(|e| format!("batch promote failed: {e}"))? as i64;

    // Backfill any done tasks missing validated_at
    conn.execute(
        "UPDATE tasks SET validated_at = datetime('now'), validated_by = ?1
         WHERE wave_id_fk = ?2 AND status = 'done' AND validated_at IS NULL",
        params![validated_by, wave_db_id],
    )
    .map_err(|e| format!("backfill validated_at failed: {e}"))?;

    // Verify all tasks now have validated_at (post-promotion sanity check)
    let missing_stamps: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks
             WHERE wave_id_fk = ?1 AND status = 'done' AND validated_at IS NULL",
            params![wave_db_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("stamp check failed: {e}"))?;

    if missing_stamps > 0 {
        return Err(format!(
            "wave {wave_db_id}: {missing_stamps} done task(s) still missing validated_at after promotion"
        ));
    }

    // Recount tasks_done for wave
    let total_done: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE wave_id_fk = ?1 AND status = 'done'",
            params![wave_db_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("count tasks_done failed: {e}"))?;

    // Mark wave done and refresh counter
    conn.execute(
        "UPDATE waves SET status = 'done', completed_at = datetime('now'), tasks_done = ?1 WHERE id = ?2",
        params![total_done, wave_db_id],
    )
    .map_err(|e| format!("update wave status failed: {e}"))?;

    // Recount plan tasks_done from all done tasks in the plan
    let plan_id: Option<i64> = conn
        .query_row(
            "SELECT plan_id FROM waves WHERE id = ?1",
            params![wave_db_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("get plan_id failed: {e}"))?;

    if let Some(pid) = plan_id {
        let plan_done: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE plan_id = ?1 AND status = 'done'",
                params![pid],
                |row| row.get(0),
            )
            .map_err(|e| format!("count plan tasks_done failed: {e}"))?;

        conn.execute(
            "UPDATE plans SET tasks_done = ?1 WHERE id = ?2",
            params![plan_done, pid],
        )
        .map_err(|e| format!("update plan tasks_done failed: {e}"))?;
    }

    Ok(ValidateWaveResult {
        wave_db_id,
        tasks_validated,
        wave_status: "done".to_string(),
    })
}

/// Enforce sequential wave execution: all waves at positions < wave_position must be done.
/// Returns Err if any predecessor wave is not in 'done' status.
pub fn check_wave_sequential(
    plan_id: i64,
    wave_position: i64,
    pool: &ConnPool,
) -> Result<(), String> {
    let conn = pool.get().map_err(pool_err)?;
    let mut stmt = conn
        .prepare(
            "SELECT position, wave_id, status FROM waves
             WHERE plan_id = ?1 AND position < ?2
             ORDER BY position ASC",
        )
        .map_err(|e| format!("prepare sequential check: {e}"))?;

    let rows: Vec<(i64, String, String)> = stmt
        .query_map(params![plan_id, wave_position], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| format!("query waves: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect waves: {e}"))?;

    for (pos, wid, status) in &rows {
        if status != "done" {
            return Err(format!(
                "Wave {wid} (position {pos}) must be completed before starting wave at \
                 position {wave_position}; current status: '{status}'"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
