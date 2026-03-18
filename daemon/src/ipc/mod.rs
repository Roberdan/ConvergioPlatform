// Plan 633: Core Engine
pub mod cli;
pub mod client;
pub mod engine;
pub mod protocol;
pub mod schema;
pub mod socket;

pub use cli::IpcArgs;
pub use engine::IpcEngine;
pub use protocol::{
    decode_request, decode_response, encode_request, encode_response, read_ipc_frame,
    write_ipc_frame, AgentInfo, ChannelInfo, ContextEntry, IpcRequest, IpcResponse, MessageInfo,
};
pub use schema::{ensure_ipc_schema, IPC_TABLES};

// Plan 634: Coordination layer
pub mod conflicts;
pub mod locks;
pub mod worktrees;

pub use conflicts::{detect_conflicts, ConflictInfo, RiskLevel};
pub use locks::{AcquireResult, IpcFileLock, IpcLockStore};
pub use worktrees::{IpcWorktree, IpcWorktreeRegistry};

// Plan 635: Intelligence layer
pub mod auth_sync;
pub mod budget;
pub mod models;
pub mod router;
pub mod skills;
