use super::super::protocol::{AgentInfo, IpcResponse};
use super::core::IpcEngine;

impl IpcEngine {
    pub fn register(
        &self,
        name: &str,
        agent_type: &str,
        pid: Option<u32>,
        host: &str,
        metadata: Option<&str>,
        parent_agent: Option<&str>,
    ) -> rusqlite::Result<IpcResponse> {
        // Agent name identifies the executor in the bus — blank names break routing.
        debug_assert!(!name.is_empty(), "register: agent name must not be empty");
        // Agent type categorises the role (e.g. "claude", "copilot") — must be specified.
        debug_assert!(!agent_type.is_empty(), "register: agent_type must not be empty");
        // Host ties the agent to a machine; empty host breaks deduplication.
        debug_assert!(!host.is_empty(), "register: host must not be empty");
        let conn = self.open_conn()?;
        conn.execute(
            "INSERT INTO ipc_agents (name, host, agent_type, pid, metadata, parent_agent, registered_at, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%f','now'), strftime('%Y-%m-%dT%H:%M:%f','now'))
             ON CONFLICT(name, host) DO UPDATE SET
               agent_type = excluded.agent_type,
               pid = excluded.pid,
               metadata = excluded.metadata,
               parent_agent = excluded.parent_agent,
               last_seen = strftime('%Y-%m-%dT%H:%M:%f','now')",
            rusqlite::params![name, host, agent_type, pid, metadata, parent_agent],
        )?;
        Ok(IpcResponse::Ok {
            message: format!("registered {name}@{host}"),
        })
    }

    pub fn unregister(&self, name: &str, host: &str) -> rusqlite::Result<IpcResponse> {
        // Same invariants as register: must identify a specific agent on a specific host.
        debug_assert!(!name.is_empty(), "unregister: agent name must not be empty");
        debug_assert!(!host.is_empty(), "unregister: host must not be empty");
        let conn = self.open_conn()?;
        let deleted = conn.execute(
            "DELETE FROM ipc_agents WHERE name = ?1 AND host = ?2",
            rusqlite::params![name, host],
        )?;
        if deleted > 0 {
            Ok(IpcResponse::Ok {
                message: format!("unregistered {name}@{host}"),
            })
        } else {
            Ok(IpcResponse::Error {
                code: 404,
                message: format!("agent {name}@{host} not found"),
            })
        }
    }

    pub fn who(&self) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let mut stmt = conn.prepare(
            "SELECT name, host, agent_type, pid, last_seen, parent_agent FROM ipc_agents ORDER BY name, host",
        )?;
        let agents: Vec<AgentInfo> = stmt
            .query_map([], |row| {
                Ok(AgentInfo {
                    name: row.get(0)?,
                    host: row.get(1)?,
                    agent_type: row.get(2)?,
                    pid: row.get(3)?,
                    last_seen: row.get(4)?,
                    parent_agent: row.get(5)?,
                })
            })?
            .filter_map(|r| match r {
                Ok(v) => Some(v),
                Err(e) => { tracing::warn!("who: skipping agent row: {e}"); None }
            })
            .collect();
        Ok(IpcResponse::AgentList { agents })
    }

    pub fn prune(&self) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let mut stmt =
            conn.prepare("SELECT name, host, pid FROM ipc_agents WHERE pid IS NOT NULL")?;
        let agents: Vec<(String, String, u32)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .filter_map(|r| match r {
                Ok(v) => Some(v),
                Err(e) => { tracing::warn!("prune: skipping agent row: {e}"); None }
            })
            .collect();

        let local_host = Self::hostname();
        let mut pruned = 0u32;
        for (name, host, pid) in &agents {
            if *host != local_host {
                continue;
            }
            #[cfg(unix)]
            {
                let alive = unsafe { libc::kill(*pid as i32, 0) } == 0;
                if !alive {
                    conn.execute(
                        "DELETE FROM ipc_agents WHERE name = ?1 AND host = ?2",
                        rusqlite::params![name, host],
                    )?;
                    pruned += 1;
                }
            }
        }
        Ok(IpcResponse::Ok {
            message: format!("pruned {pruned} dead agent(s)"),
        })
    }

    /// Remove agents whose `last_seen` is older than `ttl_secs` seconds.
    ///
    /// This evicts stale remote agents that no longer heartbeat. Local agents
    /// are kept alive through `heartbeat_local_agents`, so they will typically
    /// have a recent `last_seen` and survive the TTL check.
    pub fn prune_stale(&self, ttl_secs: u64) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let pruned = conn.execute(
            "DELETE FROM ipc_agents WHERE last_seen < strftime('%Y-%m-%dT%H:%M:%f', 'now', printf('-%d seconds', ?1))",
            rusqlite::params![ttl_secs],
        )?;
        Ok(IpcResponse::Ok {
            message: format!("pruned {pruned} stale agent(s)"),
        })
    }

    pub fn heartbeat_local_agents(&self) -> Result<usize, super::super::error::IpcError> {
        let conn = self.open_conn()?;
        let local_host = Self::hostname();
        let mut stmt =
            conn.prepare("SELECT name, pid FROM ipc_agents WHERE host = ?1 AND pid IS NOT NULL")?;
        let agents: Vec<(String, u32)> = stmt
            .query_map(rusqlite::params![local_host], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .filter_map(|r| match r {
                Ok(v) => Some(v),
                Err(e) => { tracing::warn!("heartbeat_local_agents: skipping row: {e}"); None }
            })
            .collect();

        let mut alive = 0usize;
        for (name, pid) in &agents {
            #[cfg(unix)]
            {
                let is_alive = unsafe { libc::kill(*pid as i32, 0) } == 0;
                if is_alive {
                    if let Err(e) = conn.execute(
                        "UPDATE ipc_agents SET last_seen = strftime('%Y-%m-%dT%H:%M:%f','now') WHERE name = ?1 AND host = ?2",
                        rusqlite::params![name, local_host],
                    ) {
                        tracing::warn!("heartbeat_local_agents: update last_seen failed: {e}");
                    }
                    alive += 1;
                } else {
                    if let Err(e) = conn.execute(
                        "DELETE FROM ipc_agents WHERE name = ?1 AND host = ?2",
                        rusqlite::params![name, local_host],
                    ) {
                        tracing::warn!("heartbeat_local_agents: delete dead agent failed: {e}");
                    }
                }
            }
            #[cfg(not(unix))]
            {
                drop(name);
                drop(pid);
                alive += 1;
            }
        }
        Ok(alive)
    }
}
