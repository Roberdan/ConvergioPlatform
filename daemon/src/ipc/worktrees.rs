use rusqlite::{params, Connection};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcWorktree {
    pub agent: String,
    pub host: String,
    pub branch: String,
    pub path: String,
    pub registered_at: String,
}

pub struct IpcWorktreeRegistry {
    conn: Connection,
}

impl IpcWorktreeRegistry {
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn set_worktree(
        &self,
        agent: &str,
        host: &str,
        branch: &str,
        path: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO ipc_worktrees(agent, host, branch, path)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(agent, host) DO UPDATE SET
               branch = excluded.branch,
               path = excluded.path,
               registered_at = datetime('now')",
            params![agent, host, branch, path],
        )?;
        Ok(())
    }

    pub fn remove_worktree(&self, agent: &str, host: &str) -> rusqlite::Result<usize> {
        self.conn.execute(
            "DELETE FROM ipc_worktrees WHERE agent = ?1 AND host = ?2",
            params![agent, host],
        )
    }

    pub fn list_worktrees(&self) -> rusqlite::Result<Vec<IpcWorktree>> {
        let mut stmt = self.conn.prepare(
            "SELECT agent, host, branch, path, registered_at
             FROM ipc_worktrees ORDER BY registered_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(IpcWorktree {
                agent: row.get(0)?,
                host: row.get(1)?,
                branch: row.get(2)?,
                path: row.get(3)?,
                registered_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    fn init_schema(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ipc_worktrees (
                agent TEXT NOT NULL,
                host TEXT NOT NULL,
                branch TEXT NOT NULL,
                path TEXT NOT NULL,
                registered_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (agent, host)
            );
            CREATE INDEX IF NOT EXISTS idx_ipc_worktrees_branch
                ON ipc_worktrees(branch);",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_worktree_and_list() {
        let reg = IpcWorktreeRegistry::open_in_memory().unwrap();
        reg.set_worktree("agent-a", "host1", "feature/ipc", "/tmp/wt1")
            .unwrap();
        let wts = reg.list_worktrees().unwrap();
        assert_eq!(wts.len(), 1);
        assert_eq!(wts[0].agent, "agent-a");
        assert_eq!(wts[0].branch, "feature/ipc");
    }

    #[test]
    fn upsert_updates_branch_and_path() {
        let reg = IpcWorktreeRegistry::open_in_memory().unwrap();
        reg.set_worktree("agent-a", "host1", "feature/old", "/tmp/old")
            .unwrap();
        reg.set_worktree("agent-a", "host1", "feature/new", "/tmp/new")
            .unwrap();
        let wts = reg.list_worktrees().unwrap();
        assert_eq!(wts.len(), 1);
        assert_eq!(wts[0].branch, "feature/new");
        assert_eq!(wts[0].path, "/tmp/new");
    }

    #[test]
    fn remove_worktree_deletes_entry() {
        let reg = IpcWorktreeRegistry::open_in_memory().unwrap();
        reg.set_worktree("agent-a", "host1", "main", "/tmp/wt")
            .unwrap();
        let removed = reg.remove_worktree("agent-a", "host1").unwrap();
        assert_eq!(removed, 1);
        assert!(reg.list_worktrees().unwrap().is_empty());
    }

    #[test]
    fn list_returns_multiple_entries() {
        let reg = IpcWorktreeRegistry::open_in_memory().unwrap();
        reg.set_worktree("agent-a", "host1", "main", "/tmp/a")
            .unwrap();
        reg.set_worktree("agent-b", "host2", "dev", "/tmp/b")
            .unwrap();
        let wts = reg.list_worktrees().unwrap();
        assert_eq!(wts.len(), 2);
    }
}
