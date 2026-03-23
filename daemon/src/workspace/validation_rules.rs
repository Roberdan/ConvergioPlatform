// Workspace validation rules — validate_wave + check_wave_sequential.
// Extracted from validation.rs (Plan F, T5-01).

use crate::server::state_init::ConnPool;
use crate::workspace::validation::{ValidateWaveResult, AUTHORISED_VALIDATORS};
use rusqlite::params;

fn pool_err(e: r2d2::Error) -> String {
    format!("pool error: {e}")
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
