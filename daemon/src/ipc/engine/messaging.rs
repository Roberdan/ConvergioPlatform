use std::time::Duration;

use super::core::IpcEngine;
use super::super::protocol::{IpcResponse, MessageInfo};

impl IpcEngine {
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

        let deadline = Duration::from_secs(timeout_secs);
        let notified = self.notify.clone();

        let agent_owned = agent.to_string();
        let from_owned = from_filter.map(|s| s.to_string());
        let ch_owned = channel_filter.map(|s| s.to_string());

        match tokio::time::timeout(deadline, async {
            loop {
                notified.notified().await;
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
}
