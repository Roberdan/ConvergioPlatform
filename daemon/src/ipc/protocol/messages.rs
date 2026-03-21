use serde::{Deserialize, Serialize};

pub const IPC_PROTOCOL_VERSION: u8 = 0x01;

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
