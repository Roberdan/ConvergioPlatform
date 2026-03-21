use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::Connection;
use tokio::sync::Notify;

use super::super::protocol::IpcResponse;
use super::super::schema::ensure_ipc_schema;

pub const DEFAULT_RATE_LIMIT: u32 = 100; // msgs/min

pub struct IpcEngine {
    pub db_path: PathBuf,
    pub notify: Arc<Notify>,
    pub(super) start_time: std::time::Instant,
    pub(super) rate_limit: u32,
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

    pub(super) fn host_short() -> String {
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

    pub(super) fn check_rate_limit(
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

    pub fn ping(&self) -> IpcResponse {
        IpcResponse::Pong {
            uptime_secs: self.uptime_secs(),
        }
    }

    pub fn status(&self) -> rusqlite::Result<IpcResponse> {
        self.db_stats()
    }
}
