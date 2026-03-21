use super::core::IpcEngine;
use super::super::protocol::{AgentInfo, IpcResponse};

impl IpcEngine {
    pub fn register(
        &self,
        name: &str,
        agent_type: &str,
        pid: Option<u32>,
        host: &str,
        metadata: Option<&str>,
    ) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        conn.execute(
            "INSERT INTO ipc_agents (name, host, agent_type, pid, metadata, registered_at, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, strftime('%Y-%m-%dT%H:%M:%f','now'), strftime('%Y-%m-%dT%H:%M:%f','now'))
             ON CONFLICT(name, host) DO UPDATE SET
               agent_type = excluded.agent_type,
               pid = excluded.pid,
               metadata = excluded.metadata,
               last_seen = strftime('%Y-%m-%dT%H:%M:%f','now')",
            rusqlite::params![name, host, agent_type, pid, metadata],
        )?;
        Ok(IpcResponse::Ok {
            message: format!("registered {name}@{host}"),
        })
    }

    pub fn unregister(&self, name: &str, host: &str) -> rusqlite::Result<IpcResponse> {
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
            "SELECT name, host, agent_type, pid, last_seen FROM ipc_agents ORDER BY name, host",
        )?;
        let agents: Vec<AgentInfo> = stmt
            .query_map([], |row| {
                Ok(AgentInfo {
                    name: row.get(0)?,
                    host: row.get(1)?,
                    agent_type: row.get(2)?,
                    pid: row.get(3)?,
                    last_seen: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(IpcResponse::AgentList { agents })
    }

    pub fn prune(&self) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let mut stmt =
            conn.prepare("SELECT name, host, pid FROM ipc_agents WHERE pid IS NOT NULL")?;
        let agents: Vec<(String, String, u32)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .filter_map(|r| r.ok())
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

    pub fn heartbeat_local_agents(&self) -> Result<usize, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let local_host = Self::hostname();
        let mut stmt = conn
            .prepare("SELECT name, pid FROM ipc_agents WHERE host = ?1 AND pid IS NOT NULL")
            .map_err(|e| e.to_string())?;
        let agents: Vec<(String, u32)> = stmt
            .query_map(rusqlite::params![local_host], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let mut alive = 0usize;
        for (name, pid) in &agents {
            #[cfg(unix)]
            {
                let is_alive = unsafe { libc::kill(*pid as i32, 0) } == 0;
                if is_alive {
                    conn.execute(
                        "UPDATE ipc_agents SET last_seen = strftime('%Y-%m-%dT%H:%M:%f','now') WHERE name = ?1 AND host = ?2",
                        rusqlite::params![name, local_host],
                    ).ok();
                    alive += 1;
                } else {
                    conn.execute(
                        "DELETE FROM ipc_agents WHERE name = ?1 AND host = ?2",
                        rusqlite::params![name, local_host],
                    )
                    .ok();
                }
            }
            #[cfg(not(unix))]
            {
                let _ = (name, pid);
                alive += 1;
            }
        }
        Ok(alive)
    }
}
