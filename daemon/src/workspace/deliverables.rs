// Deliverable workspace support — non-code workspaces for document/asset deliverables.
// Why: Plan B deliverables (reports, designs, docs) need workspace tracking without git
//      branches; they use a project's output_path instead of a git worktree.
use crate::server::state_init::ConnPool;
use crate::workspace::core::{generate_workspace_id, WorkspaceError, WorkspaceInfo};
use rusqlite::params;
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, WorkspaceError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverableInfo {
    pub id: i64,
    pub name: String,
    pub output_type: String,
    pub status: String,
    pub version: i64,
    pub output_path: Option<String>,
}

/// Create a non-code workspace for deliverable output.
pub fn create_deliverable_workspace(
    project_id: &str,
    task_id: Option<i64>,
    pool: &ConnPool,
) -> Result<WorkspaceInfo> {
    let conn = pool.get()?;

    let output_path: Option<String> = conn
        .query_row(
            "SELECT output_path FROM projects WHERE id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .unwrap_or(None);

    let workspace_path = output_path.unwrap_or_else(|| format!("/tmp/deliverables/{project_id}"));
    let workspace_id = generate_workspace_id();

    conn.execute(
        "INSERT INTO workspaces (workspace_id, path, branch, status) \
         VALUES (?1, ?2, NULL, 'active')",
        params![workspace_id, workspace_path],
    )?;

    let created_at: String = conn.query_row(
        "SELECT created_at FROM workspaces WHERE workspace_id = ?1",
        params![workspace_id],
        |row| row.get(0),
    )?;

    if let Some(tid) = task_id {
        conn.execute(
            "INSERT INTO workspace_events \
             (workspace_id, agent, action, file_path, detail, metadata) \
             VALUES (?1, 'system', 'workspace_created', NULL, 'deliverable workspace', ?2)",
            params![workspace_id, format!(r#"{{"task_id":{tid}}}"#)],
        )?;
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
pub fn record_deliverable_event(
    workspace_id: &str,
    deliverable_id: i64,
    action: &str,
    detail: &str,
    pool: &ConnPool,
) -> Result<i64> {
    let conn = pool.get()?;
    let metadata = format!(r#"{{"deliverable_id":{deliverable_id}}}"#);
    conn.execute(
        "INSERT INTO workspace_events \
         (workspace_id, agent, action, file_path, detail, metadata) \
         VALUES (?1, 'deliverable-agent', ?2, NULL, ?3, ?4)",
        params![workspace_id, action, detail, metadata],
    )?;
    Ok(conn.last_insert_rowid())
}

/// List deliverables associated with a workspace.
pub fn list_workspace_deliverables(
    workspace_id: &str,
    pool: &ConnPool,
) -> Result<Vec<DeliverableInfo>> {
    let conn = pool.get()?;

    let mut stmt = conn.prepare(
        "SELECT DISTINCT d.id, d.name, d.output_type, d.status, d.version, d.output_path \
             FROM workspace_events we \
             JOIN deliverables d ON \
               CAST(json_extract(we.metadata, '$.deliverable_id') AS INTEGER) = d.id \
             WHERE we.workspace_id = ?1 \
             ORDER BY d.id",
    )?;

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
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(rows)
}

#[cfg(test)]
#[path = "deliverables_tests.rs"]
mod tests;
