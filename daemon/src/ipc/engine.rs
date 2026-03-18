use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use rusqlite::Connection;
use tokio::sync::Notify;

use super::protocol::{AgentInfo, ChannelInfo, ContextEntry, IpcResponse, MessageInfo};
use super::schema::ensure_ipc_schema;

pub const DEFAULT_RATE_LIMIT: u32 = 100; // msgs/min

pub struct IpcEngine {
    pub db_path: PathBuf,
    pub notify: Arc<Notify>,
    start_time: std::time::Instant,
    rate_limit: u32,
}

impl IpcEngine {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            notify: Arc::new(Notify::new()),
            start_time: std::time::Instant::now(),
            rate_limit: DEFAULT_RATE_LIMIT,
        }
    }

    pub fn open_conn(&self) -> rusqlite::Result<Connection> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-4000;",
        )?;
        ensure_ipc_schema(&conn)?;
        Ok(conn)
    }

    pub fn hostname() -> String {
        hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".into())
    }

    fn host_short() -> String {
        let h = Self::hostname();
        h.chars().take(8).collect()
    }

    pub fn generate_msg_id() -> String {
        let host = Self::host_short();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let rnd: u32 = rand::random();
        format!("{host}-{ts:x}-{rnd:08x}")
    }

    pub fn set_rate_limit(&mut self, limit: u32) {
        self.rate_limit = limit;
    }

    fn check_rate_limit(
        &self,
        conn: &Connection,
        from: &str,
    ) -> rusqlite::Result<Option<IpcResponse>> {
        let count: u32 = conn.query_row(
            "SELECT COUNT(*) FROM ipc_messages WHERE from_agent = ?1 AND created_at > datetime('now', '-1 minute')",
            rusqlite::params![from],
            |r| r.get(0),
        )?;
        if count >= self.rate_limit {
            Ok(Some(IpcResponse::Error {
                code: 429,
                message: format!("agent '{from}' exceeded {} msgs/min", self.rate_limit),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    // ── Agent operations ─────────────────────────────

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

    // ── Messaging ────────────────────────────────────

    pub fn send_message(
        &self,
        from: &str,
        to: &str,
        content: &str,
        msg_type: &str,
        priority: i32,
    ) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        if let Some(err) = self.check_rate_limit(&conn, from)? {
            return Ok(err);
        }
        let id = Self::generate_msg_id();
        conn.execute(
            "INSERT INTO ipc_messages (id, from_agent, to_agent, content, msg_type, priority)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, from, to, content, msg_type, priority],
        )?;
        self.notify.notify_waiters();
        Ok(IpcResponse::Ok {
            message: format!("sent {id}"),
        })
    }

    pub fn broadcast(
        &self,
        from: &str,
        content: &str,
        msg_type: &str,
        channel: Option<&str>,
    ) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        if let Some(err) = self.check_rate_limit(&conn, from)? {
            return Ok(err);
        }
        let id = Self::generate_msg_id();
        conn.execute(
            "INSERT INTO ipc_messages (id, from_agent, to_agent, channel, content, msg_type)
             VALUES (?1, ?2, NULL, ?3, ?4, ?5)",
            rusqlite::params![id, from, channel, content, msg_type],
        )?;
        self.notify.notify_waiters();
        Ok(IpcResponse::Ok {
            message: format!("broadcast {id}"),
        })
    }

    pub fn receive(
        &self,
        agent: &str,
        from_filter: Option<&str>,
        channel_filter: Option<&str>,
        limit: u32,
        peek: bool,
    ) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let mut conditions = vec!["(to_agent = ?1 OR to_agent IS NULL)".to_string()];
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(agent.to_string())];

        if let Some(from) = from_filter {
            params.push(Box::new(from.to_string()));
            conditions.push(format!("from_agent = ?{}", params.len()));
        }
        if let Some(ch) = channel_filter {
            params.push(Box::new(ch.to_string()));
            conditions.push(format!("channel = ?{}", params.len()));
        }

        let where_clause = conditions.join(" AND ");
        let sql = format!(
            "SELECT id, from_agent, to_agent, channel, content, msg_type, created_at
             FROM ipc_messages WHERE {where_clause} AND read_at IS NULL
             ORDER BY created_at ASC LIMIT {limit}"
        );

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let ids_and_msgs: Vec<(String, MessageInfo)> = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    MessageInfo {
                        id: row.get(0)?,
                        from_agent: row.get(1)?,
                        to_agent: row.get(2)?,
                        channel: row.get(3)?,
                        content: row.get(4)?,
                        msg_type: row.get(5)?,
                        created_at: row.get(6)?,
                    },
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        if !peek {
            for (id, _) in &ids_and_msgs {
                conn.execute(
                    "UPDATE ipc_messages SET read_at = strftime('%Y-%m-%dT%H:%M:%f','now') WHERE id = ?1",
                    rusqlite::params![id],
                )?;
            }
        }

        let messages: Vec<MessageInfo> = ids_and_msgs.into_iter().map(|(_, m)| m).collect();
        Ok(IpcResponse::MessageList { messages })
    }

    pub async fn receive_wait(
        &self,
        agent: &str,
        from_filter: Option<&str>,
        channel_filter: Option<&str>,
        limit: u32,
        timeout_secs: u64,
    ) -> rusqlite::Result<IpcResponse> {
        // First try immediate receive
        let resp = self.receive(agent, from_filter, channel_filter, limit, false)?;
        if let IpcResponse::MessageList { ref messages } = resp {
            if !messages.is_empty() {
                return Ok(resp);
            }
        }

        // Wait for notification or timeout
        let deadline = Duration::from_secs(timeout_secs);
        let notified = self.notify.clone();

        let agent_owned = agent.to_string();
        let from_owned = from_filter.map(|s| s.to_string());
        let ch_owned = channel_filter.map(|s| s.to_string());

        match tokio::time::timeout(deadline, async {
            loop {
                notified.notified().await;
                // Re-check after notification
                let resp = self.receive(
                    &agent_owned,
                    from_owned.as_deref(),
                    ch_owned.as_deref(),
                    limit,
                    false,
                )?;
                if let IpcResponse::MessageList { ref messages } = resp {
                    if !messages.is_empty() {
                        return Ok(resp);
                    }
                }
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Ok(IpcResponse::MessageList {
                messages: Vec::new(),
            }),
        }
    }

    // ── Channels ─────────────────────────────────────

    pub fn channel_create(
        &self,
        name: &str,
        description: Option<&str>,
        created_by: &str,
    ) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO ipc_channels (name, description, created_by) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, description, created_by],
        )?;
        Ok(IpcResponse::Ok {
            message: format!("channel '{name}' created"),
        })
    }

    pub fn channel_list(&self) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let mut stmt = conn.prepare(
            "SELECT name, description, created_by, created_at FROM ipc_channels ORDER BY name",
        )?;
        let channels: Vec<ChannelInfo> = stmt
            .query_map([], |row| {
                Ok(ChannelInfo {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    created_by: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(IpcResponse::ChannelList { channels })
    }

    // ── Shared Context (LWW) ─────────────────────────

    pub fn context_get(&self, key: &str) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let result = conn.query_row(
            "SELECT key, value, version, set_by, updated_at FROM ipc_shared_context WHERE key = ?1",
            rusqlite::params![key],
            |row| {
                Ok(IpcResponse::Context {
                    key: row.get(0)?,
                    value: row.get(1)?,
                    version: row.get(2)?,
                    set_by: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        );
        match result {
            Ok(resp) => Ok(resp),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(IpcResponse::Error {
                code: 404,
                message: format!("key '{key}' not found"),
            }),
            Err(e) => Err(e),
        }
    }

    pub fn context_set(
        &self,
        key: &str,
        value: &str,
        set_by: &str,
    ) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        conn.execute(
            "INSERT INTO ipc_shared_context (key, value, version, set_by, updated_at)
             VALUES (?1, ?2, 1, ?3, strftime('%Y-%m-%dT%H:%M:%f','now'))
             ON CONFLICT(key) DO UPDATE SET
               value = excluded.value,
               version = ipc_shared_context.version + 1,
               set_by = excluded.set_by,
               updated_at = strftime('%Y-%m-%dT%H:%M:%f','now')",
            rusqlite::params![key, value, set_by],
        )?;
        Ok(IpcResponse::Ok {
            message: format!("context '{key}' set"),
        })
    }

    pub fn context_list(&self) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let mut stmt = conn.prepare(
            "SELECT key, value, version, set_by, updated_at FROM ipc_shared_context ORDER BY key",
        )?;
        let entries: Vec<ContextEntry> = stmt
            .query_map([], |row| {
                Ok(ContextEntry {
                    key: row.get(0)?,
                    value: row.get(1)?,
                    version: row.get(2)?,
                    set_by: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(IpcResponse::ContextList { entries })
    }

    pub fn context_delete(&self, key: &str) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let deleted = conn.execute(
            "DELETE FROM ipc_shared_context WHERE key = ?1",
            rusqlite::params![key],
        )?;
        if deleted > 0 {
            Ok(IpcResponse::Ok {
                message: format!("context '{key}' deleted"),
            })
        } else {
            Ok(IpcResponse::Error {
                code: 404,
                message: format!("key '{key}' not found"),
            })
        }
    }

    // ── History + DB Maintenance ─────────────────────

    pub fn history(
        &self,
        agent_filter: Option<&str>,
        channel_filter: Option<&str>,
        limit: u32,
        since: Option<&str>,
    ) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(agent) = agent_filter {
            params.push(Box::new(agent.to_string()));
            let idx = params.len();
            conditions.push(format!("(from_agent = ?{idx} OR to_agent = ?{idx})"));
        }
        if let Some(ch) = channel_filter {
            params.push(Box::new(ch.to_string()));
            conditions.push(format!("channel = ?{}", params.len()));
        }
        if let Some(ts) = since {
            params.push(Box::new(ts.to_string()));
            conditions.push(format!("created_at >= ?{}", params.len()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT id, from_agent, to_agent, channel, content, msg_type, created_at
             FROM ipc_messages {where_clause}
             ORDER BY created_at DESC LIMIT {limit}"
        );

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let messages: Vec<MessageInfo> = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(MessageInfo {
                    id: row.get(0)?,
                    from_agent: row.get(1)?,
                    to_agent: row.get(2)?,
                    channel: row.get(3)?,
                    content: row.get(4)?,
                    msg_type: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(IpcResponse::MessageList { messages })
    }

    pub fn db_stats(&self) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let agents: u64 = conn.query_row("SELECT count(*) FROM ipc_agents", [], |r| r.get(0))?;
        let messages: u64 =
            conn.query_row("SELECT count(*) FROM ipc_messages", [], |r| r.get(0))?;
        let channels: u64 =
            conn.query_row("SELECT count(*) FROM ipc_channels", [], |r| r.get(0))?;
        let context_keys: u64 =
            conn.query_row("SELECT count(*) FROM ipc_shared_context", [], |r| r.get(0))?;

        let db_size_bytes = std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(IpcResponse::Stats {
            agents,
            messages,
            channels,
            context_keys,
            db_size_bytes,
        })
    }

    pub fn db_cleanup(&self, older_than_days: u32) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let deleted = conn.execute(
            &format!(
                "DELETE FROM ipc_messages WHERE created_at < datetime('now', '-{older_than_days} days')"
            ),
            [],
        )?;
        Ok(IpcResponse::Ok {
            message: format!("cleaned up {deleted} message(s)"),
        })
    }

    pub fn db_vacuum(&self) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")?;
        Ok(IpcResponse::Ok {
            message: "vacuum complete".into(),
        })
    }

    pub fn db_reset(&self) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        for table in &super::schema::IPC_TABLES {
            conn.execute(&format!("DELETE FROM {table}"), [])?;
        }
        Ok(IpcResponse::Ok {
            message: "all IPC tables cleared".into(),
        })
    }

    pub fn ping(&self) -> IpcResponse {
        IpcResponse::Pong {
            uptime_secs: self.uptime_secs(),
        }
    }

    pub fn status(&self) -> rusqlite::Result<IpcResponse> {
        self.db_stats()
    }

    // ── Heartbeat ────────────────────────────────────

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

    // ── Dispatch ─────────────────────────────────────

    pub async fn dispatch(
        &self,
        req: super::protocol::IpcRequest,
    ) -> rusqlite::Result<IpcResponse> {
        use super::protocol::IpcRequest;
        match req {
            IpcRequest::Register {
                name,
                agent_type,
                pid,
                host,
                metadata,
            } => self.register(&name, &agent_type, pid, &host, metadata.as_deref()),
            IpcRequest::Unregister { name, host } => self.unregister(&name, &host),
            IpcRequest::Who => self.who(),
            IpcRequest::Prune => self.prune(),
            IpcRequest::Send {
                from,
                to,
                content,
                msg_type,
                priority,
            } => self.send_message(&from, &to, &content, &msg_type, priority),
            IpcRequest::Broadcast {
                from,
                content,
                msg_type,
                channel,
            } => self.broadcast(&from, &content, &msg_type, channel.as_deref()),
            IpcRequest::Receive {
                agent,
                from_filter,
                channel_filter,
                limit,
                peek,
                wait,
            } => {
                if wait {
                    self.receive_wait(
                        &agent,
                        from_filter.as_deref(),
                        channel_filter.as_deref(),
                        limit,
                        30,
                    )
                    .await
                } else {
                    self.receive(
                        &agent,
                        from_filter.as_deref(),
                        channel_filter.as_deref(),
                        limit,
                        peek,
                    )
                }
            }
            IpcRequest::ChannelCreate {
                name,
                description,
                created_by,
            } => self.channel_create(&name, description.as_deref(), &created_by),
            IpcRequest::ChannelList => self.channel_list(),
            IpcRequest::ContextGet { key } => self.context_get(&key),
            IpcRequest::ContextSet { key, value, set_by } => {
                self.context_set(&key, &value, &set_by)
            }
            IpcRequest::ContextList => self.context_list(),
            IpcRequest::ContextDelete { key } => self.context_delete(&key),
            IpcRequest::History {
                agent,
                channel,
                limit,
                since,
            } => self.history(
                agent.as_deref(),
                channel.as_deref(),
                limit,
                since.as_deref(),
            ),
            IpcRequest::DbStats => self.db_stats(),
            IpcRequest::DbCleanup { older_than_days } => self.db_cleanup(older_than_days),
            IpcRequest::DbVacuum => self.db_vacuum(),
            IpcRequest::DbReset => self.db_reset(),
            IpcRequest::Ping => Ok(self.ping()),
            IpcRequest::Status => self.status(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_engine() -> (IpcEngine, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("test-ipc.db");
        (IpcEngine::new(db), dir)
    }

    #[test]
    fn test_register_and_who() {
        let (engine, _dir) = temp_engine();
        engine
            .register("planner", "claude", Some(1234), "mac-worker-2", None)
            .unwrap();
        let resp = engine.who().unwrap();
        match resp {
            IpcResponse::AgentList { agents } => {
                assert_eq!(agents.len(), 1);
                assert_eq!(agents[0].name, "planner");
                assert_eq!(agents[0].host, "mac-worker-2");
            }
            _ => panic!("expected AgentList"),
        }
    }

    #[test]
    fn test_register_duplicate() {
        let (engine, _dir) = temp_engine();
        engine
            .register("planner", "claude", Some(1), "mac-worker-2", None)
            .unwrap();
        engine
            .register("planner", "copilot", Some(2), "mac-worker-2", None)
            .unwrap();
        match engine.who().unwrap() {
            IpcResponse::AgentList { agents } => {
                assert_eq!(agents.len(), 1);
                assert_eq!(agents[0].agent_type, "copilot"); // updated
            }
            _ => panic!("expected AgentList"),
        }
    }

    #[test]
    fn test_unregister() {
        let (engine, _dir) = temp_engine();
        engine
            .register("planner", "claude", None, "mac-worker-2", None)
            .unwrap();
        engine.unregister("planner", "mac-worker-2").unwrap();
        match engine.who().unwrap() {
            IpcResponse::AgentList { agents } => assert_eq!(agents.len(), 0),
            _ => panic!("expected AgentList"),
        }
    }

    #[test]
    fn test_send_and_receive() {
        let (engine, _dir) = temp_engine();
        engine
            .register("alice", "claude", None, "mac-worker-2", None)
            .unwrap();
        engine
            .register("bob", "claude", None, "mac-worker-2", None)
            .unwrap();
        engine
            .send_message("alice", "bob", "hello bob", "text", 0)
            .unwrap();

        match engine.receive("bob", None, None, 10, false).unwrap() {
            IpcResponse::MessageList { messages } => {
                assert_eq!(messages.len(), 1);
                assert_eq!(messages[0].content, "hello bob");
                assert_eq!(messages[0].from_agent, "alice");
            }
            _ => panic!("expected MessageList"),
        }
    }

    #[test]
    fn test_broadcast() {
        let (engine, _dir) = temp_engine();
        engine
            .broadcast("alice", "hello all", "text", None)
            .unwrap();

        match engine.receive("bob", None, None, 10, false).unwrap() {
            IpcResponse::MessageList { messages } => {
                assert_eq!(messages.len(), 1);
                assert!(messages[0].to_agent.is_none());
            }
            _ => panic!("expected MessageList"),
        }
    }

    #[test]
    fn test_receive_peek() {
        let (engine, _dir) = temp_engine();
        engine
            .send_message("alice", "bob", "peek test", "text", 0)
            .unwrap();

        // Peek should NOT mark as read
        engine.receive("bob", None, None, 10, true).unwrap();
        match engine.receive("bob", None, None, 10, false).unwrap() {
            IpcResponse::MessageList { messages } => {
                assert_eq!(messages.len(), 1, "peek should not consume message");
            }
            _ => panic!("expected MessageList"),
        }
    }

    #[test]
    fn test_channel_create_and_list() {
        let (engine, _dir) = temp_engine();
        engine
            .channel_create("general", Some("general chat"), "alice")
            .unwrap();
        engine.channel_create("ops", None, "bob").unwrap();

        match engine.channel_list().unwrap() {
            IpcResponse::ChannelList { channels } => {
                assert_eq!(channels.len(), 2);
            }
            _ => panic!("expected ChannelList"),
        }
    }

    #[test]
    fn test_context_set_get_lww() {
        let (engine, _dir) = temp_engine();
        engine.context_set("plan_id", "633", "planner").unwrap();
        engine.context_set("plan_id", "634", "executor").unwrap();

        match engine.context_get("plan_id").unwrap() {
            IpcResponse::Context {
                value,
                version,
                set_by,
                ..
            } => {
                assert_eq!(value, "634");
                assert_eq!(version, 2);
                assert_eq!(set_by, "executor");
            }
            _ => panic!("expected Context"),
        }
    }

    #[test]
    fn test_context_delete() {
        let (engine, _dir) = temp_engine();
        engine.context_set("key1", "val1", "agent").unwrap();
        engine.context_delete("key1").unwrap();

        match engine.context_get("key1").unwrap() {
            IpcResponse::Error { code, .. } => assert_eq!(code, 404),
            _ => panic!("expected Error"),
        }
    }

    #[test]
    fn test_history() {
        let (engine, _dir) = temp_engine();
        engine
            .send_message("alice", "bob", "msg1", "text", 0)
            .unwrap();
        engine.broadcast("alice", "msg2", "text", None).unwrap();

        match engine.history(Some("alice"), None, 50, None).unwrap() {
            IpcResponse::MessageList { messages } => {
                assert_eq!(messages.len(), 2);
            }
            _ => panic!("expected MessageList"),
        }
    }

    #[test]
    fn test_db_stats_and_reset() {
        let (engine, _dir) = temp_engine();
        engine.register("a", "claude", None, "h", None).unwrap();
        engine.send_message("a", "b", "x", "text", 0).unwrap();

        match engine.db_stats().unwrap() {
            IpcResponse::Stats {
                agents, messages, ..
            } => {
                assert_eq!(agents, 1);
                assert_eq!(messages, 1);
            }
            _ => panic!("expected Stats"),
        }

        engine.db_reset().unwrap();
        match engine.db_stats().unwrap() {
            IpcResponse::Stats {
                agents, messages, ..
            } => {
                assert_eq!(agents, 0);
                assert_eq!(messages, 0);
            }
            _ => panic!("expected Stats"),
        }
    }

    #[test]
    fn test_context_set_get_delete() {
        let (engine, _dir) = temp_engine();
        engine.context_set("k1", "v1", "agent").unwrap();
        match engine.context_get("k1").unwrap() {
            IpcResponse::Context { value, .. } => assert_eq!(value, "v1"),
            _ => panic!("expected Context"),
        }
        engine.context_delete("k1").unwrap();
        match engine.context_get("k1").unwrap() {
            IpcResponse::Error { code, .. } => assert_eq!(code, 404),
            _ => panic!("expected Error after delete"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_routing() {
        let (engine, _dir) = temp_engine();
        use super::super::protocol::IpcRequest;

        // Test Ping dispatch
        let resp = engine.dispatch(IpcRequest::Ping).await.unwrap();
        match resp {
            IpcResponse::Pong { .. } => {}
            _ => panic!("expected Pong"),
        }

        // Test Register + Who dispatch
        let resp = engine
            .dispatch(IpcRequest::Register {
                name: "test".into(),
                agent_type: "claude".into(),
                pid: None,
                host: "local".into(),
                metadata: None,
            })
            .await
            .unwrap();
        match resp {
            IpcResponse::Ok { .. } => {}
            _ => panic!("expected Ok"),
        }

        let resp = engine.dispatch(IpcRequest::Who).await.unwrap();
        match resp {
            IpcResponse::AgentList { agents } => assert_eq!(agents.len(), 1),
            _ => panic!("expected AgentList"),
        }
    }

    #[test]
    fn test_rate_limit() {
        let (mut engine, _dir) = temp_engine();
        engine.set_rate_limit(3);

        for i in 0..3 {
            let resp = engine
                .send_message("spammer", "bob", &format!("msg{i}"), "text", 0)
                .unwrap();
            assert!(matches!(resp, IpcResponse::Ok { .. }));
        }
        // 4th message should be rate-limited
        let resp = engine
            .send_message("spammer", "bob", "msg3", "text", 0)
            .unwrap();
        match resp {
            IpcResponse::Error { code, message } => {
                assert_eq!(code, 429);
                assert!(message.contains("exceeded"));
            }
            _ => panic!("expected rate limit error"),
        }
    }
}
