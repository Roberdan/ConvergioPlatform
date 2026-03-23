// Workspace validation rules — validate_wave + check_wave_sequential.
// Extracted from validation.rs (Plan F, T5-01).

use crate::server::state_init::ConnPool;
use crate::workspace::core::WorkspaceError;
use crate::workspace::validation::{ValidateWaveResult, AUTHORISED_VALIDATORS};
use rusqlite::params;

type Result<T> = std::result::Result<T, WorkspaceError>;

/// Validate an entire wave: batch-promote submitted->done, verify completeness, mark wave done.
pub fn validate_wave(
    wave_db_id: i64,
    validated_by: &str,
    pool: &ConnPool,
) -> Result<ValidateWaveResult> {
    if !AUTHORISED_VALIDATORS.contains(&validated_by) {
        return Err(WorkspaceError::Validation(format!(
            "validator '{validated_by}' is not authorized; must be one of: {}",
            AUTHORISED_VALIDATORS.join(", ")
        )));
    }

    let conn = pool.get()?;

    let unresolved: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks
             WHERE wave_id_fk = ?1 AND status IN ('pending', 'in_progress', 'blocked')",
        params![wave_db_id],
        |row| row.get(0),
    )?;

    if unresolved > 0 {
        return Err(WorkspaceError::Validation(format!(
            "wave {wave_db_id} has {unresolved} unresolved task(s) \
             (pending/in_progress/blocked); resolve all tasks before validating wave"
        )));
    }

    let tasks_validated = conn.execute(
        "UPDATE tasks
             SET status = 'done', validated_at = datetime('now'), validated_by = ?1,
                 completed_at = COALESCE(completed_at, datetime('now'))
             WHERE wave_id_fk = ?2 AND status = 'submitted'",
        params![validated_by, wave_db_id],
    )? as i64;

    conn.execute(
        "UPDATE tasks SET validated_at = datetime('now'), validated_by = ?1
         WHERE wave_id_fk = ?2 AND status = 'done' AND validated_at IS NULL",
        params![validated_by, wave_db_id],
    )?;

    let missing_stamps: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks
             WHERE wave_id_fk = ?1 AND status = 'done' AND validated_at IS NULL",
        params![wave_db_id],
        |row| row.get(0),
    )?;

    if missing_stamps > 0 {
        return Err(WorkspaceError::Validation(format!(
            "wave {wave_db_id}: {missing_stamps} done task(s) still missing validated_at"
        )));
    }

    let total_done: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE wave_id_fk = ?1 AND status = 'done'",
        params![wave_db_id],
        |row| row.get(0),
    )?;

    conn.execute(
        "UPDATE waves SET status = 'done', completed_at = datetime('now'), tasks_done = ?1 \
         WHERE id = ?2",
        params![total_done, wave_db_id],
    )?;

    let plan_id: Option<i64> = conn.query_row(
        "SELECT plan_id FROM waves WHERE id = ?1",
        params![wave_db_id],
        |row| row.get(0),
    )?;

    if let Some(pid) = plan_id {
        let plan_done: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE plan_id = ?1 AND status = 'done'",
            params![pid],
            |row| row.get(0),
        )?;

        conn.execute(
            "UPDATE plans SET tasks_done = ?1 WHERE id = ?2",
            params![plan_done, pid],
        )?;
    }

    Ok(ValidateWaveResult {
        wave_db_id,
        tasks_validated,
        wave_status: "done".to_string(),
    })
}

/// Enforce sequential wave execution: all waves at positions < wave_position must be done.
pub fn check_wave_sequential(plan_id: i64, wave_position: i64, pool: &ConnPool) -> Result<()> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT position, wave_id, status FROM waves
             WHERE plan_id = ?1 AND position < ?2
             ORDER BY position ASC",
    )?;

    let rows: Vec<(i64, String, String)> = stmt
        .query_map(params![plan_id, wave_position], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    for (pos, wid, status) in &rows {
        if status != "done" {
            return Err(WorkspaceError::Validation(format!(
                "Wave {wid} (position {pos}) must be completed before starting wave at \
                 position {wave_position}; current status: '{status}'"
            )));
        }
    }
    Ok(())
}
