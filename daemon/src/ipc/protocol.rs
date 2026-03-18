use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const IPC_PROTOCOL_VERSION: u8 = 0x01;
const MAX_FRAME_SIZE: u32 = 4 * 1024 * 1024; // 4 MB

// ── Request ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum IpcRequest {
    Register {
        name: String,
        agent_type: String,
        pid: Option<u32>,
        host: String,
        metadata: Option<String>,
    },
    Unregister {
        name: String,
        host: String,
    },
    Who,
    Prune,

    Send {
        from: String,
        to: String,
        content: String,
        #[serde(default = "default_msg_type")]
        msg_type: String,
        #[serde(default)]
        priority: i32,
    },
    Broadcast {
        from: String,
        content: String,
        #[serde(default = "default_msg_type")]
        msg_type: String,
        channel: Option<String>,
    },
    Receive {
        agent: String,
        #[serde(default)]
        from_filter: Option<String>,
        #[serde(default)]
        channel_filter: Option<String>,
        #[serde(default = "default_limit")]
        limit: u32,
        #[serde(default)]
        peek: bool,
        #[serde(default)]
        wait: bool,
    },

    ChannelCreate {
        name: String,
        description: Option<String>,
        created_by: String,
    },
    ChannelList,

    ContextGet {
        key: String,
    },
    ContextSet {
        key: String,
        value: String,
        set_by: String,
    },
    ContextList,
    ContextDelete {
        key: String,
    },

    History {
        agent: Option<String>,
        channel: Option<String>,
        #[serde(default = "default_history_limit")]
        limit: u32,
        since: Option<String>,
    },

    DbStats,
    DbCleanup {
        older_than_days: u32,
    },
    DbVacuum,
    DbReset,

    Ping,
    Status,
}

fn default_msg_type() -> String {
    "text".into()
}
fn default_limit() -> u32 {
    20
}
fn default_history_limit() -> u32 {
    50
}

// ── Response ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IpcResponse {
    Ok {
        message: String,
    },
    Agent {
        name: String,
        host: String,
        agent_type: String,
        pid: Option<u32>,
        last_seen: String,
    },
    AgentList {
        agents: Vec<AgentInfo>,
    },
    Message {
        id: String,
        from_agent: String,
        to_agent: Option<String>,
        channel: Option<String>,
        content: String,
        msg_type: String,
        created_at: String,
    },
    MessageList {
        messages: Vec<MessageInfo>,
    },
    Channel {
        name: String,
        description: Option<String>,
        created_by: String,
        created_at: String,
    },
    ChannelList {
        channels: Vec<ChannelInfo>,
    },
    Context {
        key: String,
        value: String,
        version: i64,
        set_by: String,
        updated_at: String,
    },
    ContextList {
        entries: Vec<ContextEntry>,
    },
    Stats {
        agents: u64,
        messages: u64,
        channels: u64,
        context_keys: u64,
        db_size_bytes: u64,
    },
    Pong {
        uptime_secs: u64,
    },
    Error {
        code: u16,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    pub host: String,
    pub agent_type: String,
    pub pid: Option<u32>,
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageInfo {
    pub id: String,
    pub from_agent: String,
    pub to_agent: Option<String>,
    pub channel: Option<String>,
    pub content: String,
    pub msg_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub name: String,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub key: String,
    pub value: String,
    pub version: i64,
    pub set_by: String,
    pub updated_at: String,
}

// ── Wire I/O ─────────────────────────────────────────────

pub async fn write_ipc_frame<W: AsyncWriteExt + Unpin>(
    w: &mut W,
    payload: &[u8],
) -> std::io::Result<()> {
    let len = payload.len() as u32;
    if len > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("frame too large: {len} > {MAX_FRAME_SIZE}"),
        ));
    }
    w.write_u8(IPC_PROTOCOL_VERSION).await?;
    w.write_u32(len).await?;
    w.write_all(payload).await?;
    w.flush().await
}

pub async fn read_ipc_frame<R: AsyncReadExt + Unpin>(r: &mut R) -> std::io::Result<Vec<u8>> {
    let version = r.read_u8().await?;
    if version != IPC_PROTOCOL_VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unsupported IPC protocol version: {version:#04x}, expected {IPC_PROTOCOL_VERSION:#04x}"),
        ));
    }
    let len = r.read_u32().await?;
    if len > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("frame too large: {len} > {MAX_FRAME_SIZE}"),
        ));
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf).await?;
    Ok(buf)
}

pub fn encode_request(req: &IpcRequest) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    rmp_serde::to_vec_named(req)
}

pub fn decode_request(data: &[u8]) -> Result<IpcRequest, rmp_serde::decode::Error> {
    rmp_serde::from_slice(data)
}

pub fn encode_response(resp: &IpcResponse) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    rmp_serde::to_vec_named(resp)
}

pub fn decode_response(data: &[u8]) -> Result<IpcResponse, rmp_serde::decode::Error> {
    rmp_serde::from_slice(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_roundtrip() {
        let req = IpcRequest::Register {
            name: "planner".into(),
            agent_type: "claude".into(),
            pid: Some(1234),
            host: "mac-worker-2".into(),
            metadata: None,
        };
        let encoded = encode_request(&req).unwrap();
        let decoded = decode_request(&encoded).unwrap();
        match decoded {
            IpcRequest::Register { name, host, .. } => {
                assert_eq!(name, "planner");
                assert_eq!(host, "mac-worker-2");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_response_roundtrip() {
        let resp = IpcResponse::Pong { uptime_secs: 42 };
        let encoded = encode_response(&resp).unwrap();
        let decoded = decode_response(&encoded).unwrap();
        match decoded {
            IpcResponse::Pong { uptime_secs } => assert_eq!(uptime_secs, 42),
            _ => panic!("wrong variant"),
        }
    }

    #[tokio::test]
    async fn test_frame_roundtrip() {
        let payload = b"hello ipc";
        let mut buf = Vec::new();
        write_ipc_frame(&mut buf, payload).await.unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let read_back = read_ipc_frame(&mut cursor).await.unwrap();
        assert_eq!(read_back, payload);
    }

    #[tokio::test]
    async fn test_frame_rejects_bad_version() {
        let mut buf = vec![0x99, 0, 0, 0, 1, 0x42]; // version 0x99
        let mut cursor = std::io::Cursor::new(&mut buf);
        let err = read_ipc_frame(&mut cursor).await.unwrap_err();
        assert!(err.to_string().contains("unsupported IPC protocol version"));
    }

    #[tokio::test]
    async fn test_protocol_version_reject() {
        // Version 0x02 should be rejected
        let mut buf = vec![0x02, 0, 0, 0, 1, 0x00];
        let mut cursor = std::io::Cursor::new(&mut buf);
        let err = read_ipc_frame(&mut cursor).await.unwrap_err();
        assert!(err.to_string().contains("unsupported IPC protocol version"));
        assert!(err.to_string().contains("0x02"));
    }
}
