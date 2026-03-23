// Deliverable workspace support — non-code workspaces for document/asset deliverables.
// Why: Plan B deliverables (reports, designs, docs) need workspace tracking without git
//      branches; they use a project's output_path instead of a git worktree.
use crate::server::state_init::ConnPool;
use crate::workspace::core::{generate_workspace_id, WorkspaceInfo};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverableInfo {
    pub id: i64,
    pub name: String,
    pub output_type: String,
    pub status: String,
    pub version: i64,
    pub output_path: Option<String>,
}

fn pool_err(e: r2d2::Error) -> String {
    format!("pool error: {e}")
}

fn db_err(e: rusqlite::Error) -> String {
    format!("db error: {e}")
}

/// Create a non-code workspace for deliverable output.
/// Uses project.output_path as the workspace path; no git branch required.
pub fn create_deliverable_workspace(
    project_id: &str,
    task_id: Option<i64>,
    pool: &ConnPool,
) -> Result<WorkspaceInfo, String> {
    let conn = pool.get().map_err(pool_err)?;

    // Resolve project output_path — fallback to empty string if not configured
    let output_path: Option<String> = conn
        .query_row(
            "SELECT output_path FROM projects WHERE id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .unwrap_or(None);

    let workspace_path = output_path.unwrap_or_else(|| format!("/tmp/deliverables/{project_id}"));
    let workspace_id = generate_workspace_id();

    // Workspace has no branch — NULL signals non-code workspace
    conn.execute(
        "INSERT INTO workspaces (workspace_id, path, branch, status) \
         VALUES (?1, ?2, NULL, 'active')",
        params![workspace_id, workspace_path],
    )
    .map_err(db_err)?;

    let created_at: String = conn
        .query_row(
            "SELECT created_at FROM workspaces WHERE workspace_id = ?1",
            params![workspace_id],
            |row| row.get(0),
        )
        .map_err(db_err)?;

    // If task_id provided, record which task owns this deliverable workspace
    if let Some(tid) = task_id {
        conn.execute(
            "INSERT INTO workspace_events \
             (workspace_id, agent, action, file_path, detail, metadata) \
             VALUES (?1, 'system', 'workspace_created', NULL, 'deliverable workspace', ?2)",
            params![workspace_id, format!(r#"{{"task_id":{tid}}}"#)],
        )
        .map_err(db_err)?;
    }

    Ok(WorkspaceInfo {
        workspace_id,
        path: workspace_path,
        branch: None,
        plan_id: None,
        wave_db_id: None,
        status: "active".to_string(),
        created_at,
    })
}

/// Record a workspace event linked to a specific deliverable.
/// Stores deliverable_id in metadata JSON for traceability.
pub fn record_deliverable_event(
    workspace_id: &str,
    deliverable_id: i64,
    action: &str,
    detail: &str,
    pool: &ConnPool,
) -> Result<i64, String> {
    let conn = pool.get().map_err(pool_err)?;
    let metadata = format!(r#"{{"deliverable_id":{deliverable_id}}}"#);
    conn.execute(
        "INSERT INTO workspace_events \
         (workspace_id, agent, action, file_path, detail, metadata) \
         VALUES (?1, 'deliverable-agent', ?2, NULL, ?3, ?4)",
        params![workspace_id, action, detail, metadata],
    )
    .map_err(db_err)?;
    Ok(conn.last_insert_rowid())
}

/// List deliverables associated with a workspace.
/// Joins workspace_events (filtered by deliverable metadata) with the deliverables table.
pub fn list_workspace_deliverables(
    workspace_id: &str,
    pool: &ConnPool,
) -> Result<Vec<DeliverableInfo>, String> {
    let conn = pool.get().map_err(pool_err)?;

    // Extract deliverable_ids referenced in workspace_events metadata,
    // then join to deliverables table for full info.
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT d.id, d.name, d.output_type, d.status, d.version, d.output_path \
             FROM workspace_events we \
             JOIN deliverables d ON \
               CAST(json_extract(we.metadata, '$.deliverable_id') AS INTEGER) = d.id \
             WHERE we.workspace_id = ?1 \
             ORDER BY d.id",
        )
        .map_err(db_err)?;

    let rows = stmt
        .query_map(params![workspace_id], |row| {
            Ok(DeliverableInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                output_type: row.get(2)?,
                status: row.get(3)?,
                version: row.get(4)?,
                output_path: row.get(5)?,
            })
        })
        .map_err(db_err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(db_err)?;

    Ok(rows)
}

#[cfg(test)]
#[path = "deliverables_tests.rs"]
mod tests;
