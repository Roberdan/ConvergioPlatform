use crate::server::state_init::ConnPool;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkspaceAction {
    FileRead,
    FileWrite,
    FileEdit,
    GitCommit,
    GitPush,
    PrCreated,
    PrMerged,
    QualityGatePass,
    QualityGateFail,
    WorkspaceCreated,
    WorkspaceDeleted,
}

impl fmt::Display for WorkspaceAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::FileRead => "file_read",
            Self::FileWrite => "file_write",
            Self::FileEdit => "file_edit",
            Self::GitCommit => "git_commit",
            Self::GitPush => "git_push",
            Self::PrCreated => "pr_created",
            Self::PrMerged => "pr_merged",
            Self::QualityGatePass => "quality_gate_pass",
            Self::QualityGateFail => "quality_gate_fail",
            Self::WorkspaceCreated => "workspace_created",
            Self::WorkspaceDeleted => "workspace_deleted",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug)]
pub struct ParseActionError(String);

impl fmt::Display for ParseActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown workspace action: {}", self.0)
    }
}

impl FromStr for WorkspaceAction {
    type Err = ParseActionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "file_read" => Ok(Self::FileRead),
            "file_write" => Ok(Self::FileWrite),
            "file_edit" => Ok(Self::FileEdit),
            "git_commit" => Ok(Self::GitCommit),
            "git_push" => Ok(Self::GitPush),
            "pr_created" => Ok(Self::PrCreated),
            "pr_merged" => Ok(Self::PrMerged),
            "quality_gate_pass" => Ok(Self::QualityGatePass),
            "quality_gate_fail" => Ok(Self::QualityGateFail),
            "workspace_created" => Ok(Self::WorkspaceCreated),
            "workspace_deleted" => Ok(Self::WorkspaceDeleted),
            other => Err(ParseActionError(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEvent {
    pub id: i64,
    pub workspace_id: String,
    pub agent: String,
    pub action: String,
    pub file_path: Option<String>,
    pub detail: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
}

pub struct EventLogger {
    pool: ConnPool,
}

fn pool_err(e: r2d2::Error) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
        Some(format!("pool error: {e}")),
    )
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceEvent> {
    Ok(WorkspaceEvent {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        agent: row.get(2)?,
        action: row.get(3)?,
        file_path: row.get(4)?,
        detail: row.get(5)?,
        metadata: row.get(6)?,
        created_at: row.get(7)?,
    })
}

const SELECT_COLS: &str =
    "SELECT id, workspace_id, agent, action, file_path, detail, metadata, created_at \
     FROM workspace_events";

impl EventLogger {
    pub fn new(pool: ConnPool) -> Self {
        Self { pool }
    }

    pub fn record_event(
        &self,
        workspace_id: &str,
        agent: &str,
        action: WorkspaceAction,
        file_path: Option<&str>,
        detail: Option<&str>,
        metadata: Option<&str>,
    ) -> rusqlite::Result<i64> {
        let conn = self.pool.get().map_err(pool_err)?;
        let action_str = action.to_string();
        conn.execute(
            "INSERT INTO workspace_events \
             (workspace_id, agent, action, file_path, detail, metadata) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![workspace_id, agent, action_str, file_path, detail, metadata],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn query_events(
        &self,
        workspace_id: &str,
        limit: Option<i64>,
        since: Option<&str>,
    ) -> rusqlite::Result<Vec<WorkspaceEvent>> {
        let conn = self.pool.get().map_err(pool_err)?;
        let cap = limit.unwrap_or(100);
        if let Some(ts) = since {
            let sql = format!("{SELECT_COLS} WHERE workspace_id = ?1 AND created_at >= ?2 ORDER BY created_at DESC LIMIT ?3");
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![workspace_id, ts, cap], row_to_event)?;
            rows.collect()
        } else {
            let sql =
                format!("{SELECT_COLS} WHERE workspace_id = ?1 ORDER BY created_at DESC LIMIT ?2");
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![workspace_id, cap], row_to_event)?;
            rows.collect()
        }
    }

    pub fn query_events_by_agent(
        &self,
        agent: &str,
        limit: Option<i64>,
    ) -> rusqlite::Result<Vec<WorkspaceEvent>> {
        let conn = self.pool.get().map_err(pool_err)?;
        let cap = limit.unwrap_or(100);
        let sql = format!("{SELECT_COLS} WHERE agent = ?1 ORDER BY created_at DESC LIMIT ?2");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![agent, cap], row_to_event)?;
        rows.collect()
    }
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
