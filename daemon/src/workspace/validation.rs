// Workspace validation — replaces validate-task.sh + validate-wave.sh.
// Enforces sequential wave execution and Thor-only validation authority.
// Why: Plan 698 — centralise validation in daemon, remove bash script drift.

use crate::server::state_init::ConnPool;
use crate::workspace::core::WorkspaceError;
use rusqlite::params;
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, WorkspaceError>;

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
pub(crate) const AUTHORISED_VALIDATORS: &[&str] =
    &["thor", "thor-quality-assurance-guardian", "thor-per-wave"];

// Wave validation + sequential checks -> validation_rules.rs
pub use crate::workspace::validation_rules::{check_wave_sequential, validate_wave};

/// Validate a single task: submitted->done, or backfill validated_at on done tasks.
/// Only Thor agents may call this. Updates wave/plan tasks_done counters.
pub fn validate_task(
    task_db_id: i64,
    validated_by: &str,
    pool: &ConnPool,
) -> Result<ValidateTaskResult> {
    if !AUTHORISED_VALIDATORS.contains(&validated_by) {
        return Err(WorkspaceError::Validation(format!(
            "validator '{validated_by}' is not authorized; must be one of: {}",
            AUTHORISED_VALIDATORS.join(", ")
        )));
    }
    let conn = pool.get()?;
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
        .map_err(|e| WorkspaceError::NotFound(format!("task {task_db_id}: {e}")))?;

    match old_status.as_str() {
        "submitted" => {
            conn.execute(
                "UPDATE tasks SET status='done', validated_at=datetime('now'), validated_by=?1,
                 completed_at=COALESCE(completed_at,datetime('now')) WHERE id=?2",
                params![validated_by, task_db_id],
            )?;
            if let Some(wid) = wave_id_fk {
                conn.execute(
                    "UPDATE waves SET tasks_done=tasks_done+1 WHERE id=?1",
                    params![wid],
                )?;
            }
            if let Some(pid) = plan_id {
                conn.execute(
                    "UPDATE plans SET tasks_done=tasks_done+1 WHERE id=?1",
                    params![pid],
                )?;
            }
            Ok(ValidateTaskResult {
                task_db_id,
                old_status,
                new_status: "done".into(),
            })
        }
        "done" if validated_at.is_none() => {
            conn.execute(
                "UPDATE tasks SET validated_at=datetime('now'), validated_by=?1 WHERE id=?2",
                params![validated_by, task_db_id],
            )?;
            Ok(ValidateTaskResult {
                task_db_id,
                old_status: "done".into(),
                new_status: "done".into(),
            })
        }
        other => Err(WorkspaceError::Validation(format!(
            "task {task_db_id} has status '{other}'; must be 'submitted' or 'done' to validate"
        ))),
    }
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
