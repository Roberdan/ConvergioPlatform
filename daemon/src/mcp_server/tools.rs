// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// MCP tool registry: struct definition + ring-filtered listing.
// 20 tools including invoke_agent and chat IPC tools. Definitions in tool_catalog.rs.

use serde_json::Value;

use crate::capabilities::ring::Ring;
use crate::mcp_server::tool_catalog::all_tools;

// ── Tool definition ───────────────────────────────────────────────────────────

pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    /// Minimum ring required to use this tool.
    pub min_ring: Ring,
}

// ── Ring-filtered list ────────────────────────────────────────────────────────

/// Returns all tools visible to `caller_ring`.
/// A caller at ring N sees tools whose min_ring >= N (caller is at least as privileged).
pub fn list_tools(caller_ring: Ring) -> Vec<McpTool> {
    all_tools()
        .into_iter()
        .filter(|t| caller_ring.can_access(t.min_ring))
        .collect()
}
