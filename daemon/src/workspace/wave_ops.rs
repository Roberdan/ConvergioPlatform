// Wave workspace lifecycle — create/cleanup/status for plan waves.
// Replaces wave-worktree.sh create/cleanup commands.
// Why: centralize wave-workspace binding in daemon, avoid bash script drift (Plan 698).

use super::core::WorkspaceManager;
use crate::server::state_init::ConnPool;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveWorkspaceInfo {
    pub workspace_id: String,
    pub wave_db_id: i64,
    pub wave_id: String,
    pub plan_id: i64,
    pub path: String,
    pub branch: String,
}

pub(crate) fn pool_err(e: r2d2::Error) -> String {
    format!("pool error: {e}")
}

/// Create a workspace for a wave: git worktree + branch + DB links.
/// Branch naming: plan/{plan_id}-{wave_id} (e.g. "plan/698-W1").
pub fn create_wave_workspace(
    manager: &WorkspaceManager,
    plan_id: i64,
    wave_db_id: i64,
    pool: &ConnPool,
) -> Result<WaveWorkspaceInfo, String> {
    let conn = pool.get().map_err(pool_err)?;

    // Resolve wave_id from DB (e.g. "W1")
    let wave_id: String = conn
        .query_row(
            "SELECT wave_id FROM waves WHERE id = ?1",
            params![wave_db_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("wave {wave_db_id} not found: {e}"))?;

    let branch = format!("plan/{plan_id}-{wave_id}");

    // Delegate workspace creation to core (git worktree + DB insert)
    let info = manager
        .create_workspace(Some(plan_id), Some(wave_db_id))
        .map_err(|e| format!("create_workspace failed: {e}"))?;

    // Update waves row with worktree path + canonical branch name
    // (core creates a generic workspace branch; wave branch is the plan-scoped name)
    pool.get()
        .map_err(pool_err)?
        .execute(
            "UPDATE waves SET worktree_path = ?1, branch_name = ?2 WHERE id = ?3",
            params![info.path, branch, wave_db_id],
        )
        .map_err(|e| format!("update waves failed: {e}"))?;

    Ok(WaveWorkspaceInfo {
        workspace_id: info.workspace_id,
        wave_db_id,
        wave_id,
        plan_id,
        path: info.path,
        branch,
    })
}

/// Delete the workspace associated with a wave and clear wave DB fields.
pub fn cleanup_wave_workspace(
    manager: &WorkspaceManager,
    wave_db_id: i64,
    pool: &ConnPool,
) -> Result<(), String> {
    let conn = pool.get().map_err(pool_err)?;

    // Find workspace_id from workspaces table linked to this wave
    let workspace_id: String = conn
        .query_row(
            "SELECT workspace_id FROM workspaces WHERE wave_db_id = ?1 AND status = 'active'",
            params![wave_db_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("no active workspace for wave {wave_db_id}: {e}"))?;

    manager
        .delete_workspace(&workspace_id)
        .map_err(|e| format!("delete_workspace failed: {e}"))?;

    pool.get()
        .map_err(pool_err)?
        .execute(
            "UPDATE waves SET worktree_path = NULL, branch_name = NULL WHERE id = ?1",
            params![wave_db_id],
        )
        .map_err(|e| format!("clear wave worktree fields failed: {e}"))?;

    Ok(())
}

/// Return current workspace status for a wave, or None if no workspace assigned.
/// Checks both DB state and whether the path exists on disk.
pub fn wave_workspace_status(
    wave_db_id: i64,
    pool: &ConnPool,
) -> Result<Option<WaveWorkspaceInfo>, String> {
    let conn = pool.get().map_err(pool_err)?;

    // Join workspaces + waves to get full info
    let result: rusqlite::Result<(String, String, i64, String, String)> = conn.query_row(
        "SELECT ws.workspace_id, w.wave_id, w.plan_id, ws.path, ws.branch
         FROM waves w
         JOIN workspaces ws ON ws.wave_db_id = w.id
         WHERE w.id = ?1 AND ws.status = 'active'",
        params![wave_db_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    );

    match result {
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("status query failed: {e}")),
        Ok((workspace_id, wave_id, plan_id, path, branch)) => {
            // If path no longer exists on disk, treat as absent
            if !Path::new(&path).exists() {
                return Ok(None);
            }
            Ok(Some(WaveWorkspaceInfo {
                workspace_id,
                wave_db_id,
                wave_id,
                plan_id,
                path,
                branch,
            }))
        }
    }
}

#[cfg(test)]
#[path = "wave_ops_tests.rs"]
mod tests;
