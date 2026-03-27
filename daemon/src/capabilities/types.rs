use serde::{Deserialize, Serialize};
use thiserror::Error;

/// JSON Schema description of a tool's interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
}

/// A registered capability entry combining metadata and invocation info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub description: String,
    pub ring: u8,
    /// Optional MCP server URI for remote tools.
    pub mcp_server: Option<String>,
    pub input_schema: serde_json::Value,
    /// Permissions required to invoke this capability.
    pub permissions_required: Vec<String>,
    /// Whether the capability is currently active.
    pub enabled: bool,
}

impl Capability {
    pub fn ring_level(&self) -> super::Ring {
        super::Ring::from_u8(self.ring)
    }
}

/// Errors from the capability system.
#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("capability not found: {0}")]
    NotFound(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("invocation failed: {0}")]
    InvocationFailed(String),
    #[error("ring violation: agent ring {agent} cannot access capability ring {capability}")]
    RingViolation { agent: u8, capability: u8 },
    #[error("circuit open: {0}")]
    CircuitOpen(String),
    #[error("rate limited: {0}")]
    RateLimited(String),
}
