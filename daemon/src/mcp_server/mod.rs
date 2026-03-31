// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// McpServer: stdio loop + JSON-RPC dispatch for convergio-mcp-server.
// CRITICAL: stdout is protocol-only. All logs go to stderr via tracing.

use std::io::{BufRead, BufReader, Write};

use serde_json::{json, Value};
use tracing::warn;

use crate::capabilities::ring::Ring;
use crate::mcp_server::handlers::handle_tool_call;
use crate::mcp_server::protocol::{error_codes, JsonRpcRequest, JsonRpcResponse};
use crate::mcp_server::security::check_ring_access;
use crate::mcp_server::tools::list_tools;

pub mod agent_chat;
pub mod agent_factory;
pub mod handlers;
pub mod invoke_agent;
pub mod plan_tools;
pub mod platform_tools;
pub mod protocol;
pub mod security;
pub mod tool_catalog;
pub mod tools;
pub mod web_search;

#[cfg(test)]
mod tests;

// ── Server ────────────────────────────────────────────────────────────────────

pub struct McpServer {
    ring: Ring,
    daemon_url: String,
    api_token: Option<String>,
}

impl McpServer {
    pub fn new(ring_level: u8, daemon_url: &str, token: Option<&str>) -> Self {
        Self {
            ring: Ring::from_u8(ring_level),
            daemon_url: daemon_url.to_string(),
            api_token: token.map(|t| t.to_string()),
        }
    }

    /// Blocking stdio loop: reads JSON-RPC lines from stdin, writes responses to stdout.
    /// Returns when stdin is closed (client disconnects).
    pub fn run_stdio(&self) {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let reader = BufReader::new(stdin.lock());

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("stdin read error: {e}");
                    break;
                }
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let response = self.handle_request(trimmed);
            let mut out = stdout.lock();
            if let Err(e) = writeln!(out, "{response}") {
                eprintln!("stdout write error: {e}");
                break;
            }
            let _ = out.flush();
        }
    }

    /// Parse one JSON-RPC line and return the serialised response.
    /// Never panics — all errors are returned as JSON-RPC error responses.
    pub fn handle_request(&self, raw: &str) -> String {
        // Parse envelope
        let req: JsonRpcRequest = match serde_json::from_str(raw) {
            Ok(r) => r,
            Err(_) => {
                let resp = JsonRpcResponse::error(
                    json!(null),
                    error_codes::INVALID_REQUEST,
                    "Invalid JSON-RPC request",
                );
                return serde_json::to_string(&resp).unwrap_or_default();
            }
        };

        let id = req.id.clone().unwrap_or(json!(null));

        let resp = match req.method.as_str() {
            "initialize" => self.handle_initialize(id),
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tools_call(id, req.params.unwrap_or(json!({}))),
            _ => {
                warn!(method = %req.method, "method not found");
                JsonRpcResponse::error(id, error_codes::METHOD_NOT_FOUND, "Method not found")
            }
        };

        serde_json::to_string(&resp).unwrap_or_default()
    }

    // ── Handlers ──────────────────────────────────────────────────────────────

    fn handle_initialize(&self, id: Value) -> JsonRpcResponse {
        JsonRpcResponse::result(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {
                    "name": "convergio-mcp-server",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )
    }

    fn handle_tools_list(&self, id: Value) -> JsonRpcResponse {
        let tools: Vec<Value> = list_tools(self.ring)
            .into_iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema,
                })
            })
            .collect();
        JsonRpcResponse::result(id, json!({"tools": tools}))
    }

    fn handle_tools_call(&self, id: Value, params: Value) -> JsonRpcResponse {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => {
                return JsonRpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    "tools/call requires params.name",
                );
            }
        };
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        // Enforce ring access — look up the tool's min_ring requirement
        let tool_ring = min_ring_for_tool(name);
        if let Err(e) = check_ring_access(self.ring, tool_ring) {
            return JsonRpcResponse::error(id, e.json_rpc_code(), e.message());
        }

        match handle_tool_call(name, &args, &self.daemon_url, self.api_token.as_deref()) {
            Ok(result) => {
                // MCP spec: result.content is an array of content items
                let text = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());
                JsonRpcResponse::result(
                    id,
                    json!({
                        "content": [{"type": "text", "text": text}]
                    }),
                )
            }
            Err(e) => JsonRpcResponse::error(id, e.json_rpc_code(), e.message()),
        }
    }
}

// ── Ring map ──────────────────────────────────────────────────────────────────

/// Returns the minimum ring required to invoke a named tool.
/// Unknown tools default to Core (most restrictive) to fail-safe.
fn min_ring_for_tool(name: &str) -> Ring {
    match name {
        // Core only
        "cvg_restart_node" => Ring::Core,
        // Trusted (Ring 1)
        "cvg_update_task"
        | "cvg_checkpoint_save"
        | "cvg_agent_send"
        | "cvg_agent_ask"
        | "cvg_agent_start"
        | "cvg_agent_complete"
        | "cvg_create_agent"
        | "cvg_kernel_ask"
        | "cvg_notify"
        | "cvg_invoke_agent"
        | "cvg_create_plan"
        | "cvg_start_plan"
        | "cvg_create_task"
        | "cvg_record_validation"
        | "cvg_quality_gate" => Ring::Trusted,
        // Sandboxed (Ring 3) — accessible to all callers
        "cvg_list_plans" | "cvg_list_agents" => Ring::Sandboxed,
        // Community read-only (Ring 2)
        "cvg_get_plan" | "cvg_mesh_status" | "cvg_node_readiness" | "cvg_cost_summary"
        | "cvg_kernel_status" | "cvg_health_deep" | "cvg_list_workspaces"
        | "cvg_remember" | "cvg_recall" | "cvg_budget" | "cvg_agent_catalog"
        | "cvg_list_messages" => Ring::Community,
        // Unknown — default fail-safe to Core
        _ => Ring::Core,
    }
}
