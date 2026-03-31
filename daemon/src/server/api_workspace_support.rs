use super::state::{ApiError, ServerState};
use crate::workspace::core::{WorkspaceInfo, WorkspaceManager};
use rusqlite::OptionalExtension;
use std::env;
use std::path::PathBuf;

pub(super) fn make_manager(
    state: &ServerState,
    plan_id: Option<i64>,
    wave_db_id: Option<i64>,
) -> Result<WorkspaceManager, ApiError> {
    Ok(WorkspaceManager::new(
        state.pool(),
        resolve_repo_root(state, plan_id, wave_db_id)?,
    ))
}

fn resolve_repo_root(
    state: &ServerState,
    plan_id: Option<i64>,
    wave_db_id: Option<i64>,
) -> Result<PathBuf, ApiError> {
    let Some(plan_id) = resolve_plan_id(state, plan_id, wave_db_id)? else {
        return Ok(repo_root_from_env());
    };
    let conn = state.get_conn()?;
    let path: Option<String> = conn
        .query_row(
            "SELECT COALESCE(NULLIF(pr.path, ''), NULLIF(pr.input_path, ''), NULLIF(pr.output_path, ''))
             FROM plans p LEFT JOIN projects pr ON LOWER(pr.id) = LOWER(p.project_id) WHERE p.id = ?1",
            rusqlite::params![plan_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| ApiError::internal(format!("resolve project path failed: {e}")))?;
    path.filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            ApiError::bad_request(format!(
                "project path missing for plan {plan_id}; register or update the project before creating a workspace"
            ))
        })
}

fn resolve_plan_id(
    state: &ServerState,
    plan_id: Option<i64>,
    wave_db_id: Option<i64>,
) -> Result<Option<i64>, ApiError> {
    if plan_id.is_some() || wave_db_id.is_none() {
        return Ok(plan_id);
    }
    state
        .get_conn()?
        .query_row(
            "SELECT plan_id FROM waves WHERE id = ?1",
            rusqlite::params![wave_db_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| ApiError::internal(format!("resolve wave plan failed: {e}")))
}

fn repo_root_from_env() -> PathBuf {
    env::var("CONVERGIO_REPO_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            env::var("HOME")
                .map(|h| PathBuf::from(h).join("GitHub/ConvergioPlatform"))
                .unwrap_or_else(|_| PathBuf::from("."))
        })
}

pub(super) fn bind_workspace_context(
    conn: &rusqlite::Connection,
    ws: &WorkspaceInfo,
) -> Result<(), ApiError> {
    if let Some(plan_id) = ws.plan_id {
        super::api_plan_db_execution_context::set_worktree_in_db(conn, plan_id, &ws.path)?;
        if let Some(branch) = ws.branch.as_deref() {
            super::api_plan_db_execution_context::set_branch_in_db(conn, plan_id, branch)?;
        }
    }
    if let Some(wave_db_id) = ws.wave_db_id {
        let changed = conn
            .execute(
                "UPDATE waves SET worktree_path = ?1, branch_name = ?2 WHERE id = ?3",
                rusqlite::params![ws.path, ws.branch.as_deref().unwrap_or(""), wave_db_id],
            )
            .map_err(|e| ApiError::internal(format!("bind wave workspace failed: {e}")))?;
        if changed == 0 {
            return Err(ApiError::bad_request(format!(
                "wave {wave_db_id} not found"
            )));
        }
    }
    Ok(())
}
