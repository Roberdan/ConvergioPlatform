// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Ring-based access control for MCP tool calls.
// Lower ring number = more privilege (0=Core, 1=Trusted, 2=Community, 3=Sandboxed).

use crate::capabilities::ring::Ring;

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum McpError {
    /// Caller ring is lower privilege than the tool requires.
    RingViolation { caller: u8, required: u8 },
    /// Required parameter absent or wrong type.
    InvalidParams(&'static str),
    /// Daemon HTTP endpoint returned a non-2xx status.
    DaemonError(String),
    /// Daemon TCP connection refused or timed out.
    DaemonUnreachable,
}

impl McpError {
    pub fn json_rpc_code(&self) -> i32 {
        use crate::mcp_server::protocol::error_codes::*;
        match self {
            McpError::RingViolation { .. } => RING_VIOLATION,
            McpError::InvalidParams(_) => INVALID_PARAMS,
            McpError::DaemonError(_) => DAEMON_ERROR,
            McpError::DaemonUnreachable => DAEMON_UNREACHABLE,
        }
    }

    pub fn message(&self) -> String {
        match self {
            McpError::RingViolation { caller, required } => format!(
                "Ring violation: caller ring {} cannot access tool requiring ring {}",
                caller, required
            ),
            McpError::InvalidParams(msg) => format!("Invalid params: {msg}"),
            McpError::DaemonError(msg) => format!("Daemon error: {msg}"),
            McpError::DaemonUnreachable => {
                "Daemon unreachable at localhost:8420. Is the daemon running?".to_string()
            }
        }
    }
}

// ── Ring access check ─────────────────────────────────────────────────────────

/// Returns Ok if `caller` ring can access a tool at `required` ring level.
/// Core(0) can access everything; Sandboxed(3) can only access Sandboxed tools.
pub fn check_ring_access(caller: Ring, required: Ring) -> Result<(), McpError> {
    if caller.can_access(required) {
        Ok(())
    } else {
        Err(McpError::RingViolation {
            caller: caller.as_u8(),
            required: required.as_u8(),
        })
    }
}
