// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// MCP tool registry: 14 tool definitions with JSON Schema, filtered by ring.

use serde_json::{json, Value};

use crate::capabilities::ring::Ring;

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

// ── Full tool catalogue ───────────────────────────────────────────────────────

fn all_tools() -> Vec<McpTool> {
    vec![
        // Plans — read (Ring 3: available to all callers including sandboxed)
        McpTool {
            name: "cvg_list_plans".to_string(),
            description: "List all plans with id, name, status, tasks_done, tasks_total.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "status_filter": {
                        "type": "string",
                        "description": "Filter by status: todo, doing, done, cancelled",
                        "enum": ["todo", "doing", "done", "cancelled"]
                    }
                }
            }),
            min_ring: Ring::Sandboxed,
        },
        McpTool {
            name: "cvg_get_plan".to_string(),
            description: "Get full plan JSON with tasks, waves, and progress. Use plan_id from cvg_list_plans.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "plan_id": {"type": "integer", "description": "Plan ID"}
                },
                "required": ["plan_id"]
            }),
            min_ring: Ring::Community,
        },
        // Plans — write (Ring 1)
        McpTool {
            name: "cvg_update_task".to_string(),
            description: "Update task status. Valid transitions: pending->in_progress, in_progress->submitted, submitted->done.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": {"type": "integer", "description": "Task ID"},
                    "status": {
                        "type": "string",
                        "enum": ["in_progress", "submitted", "done", "blocked"]
                    },
                    "summary": {"type": "string", "description": "Completion summary (required for done)"}
                },
                "required": ["task_id", "status"]
            }),
            min_ring: Ring::Trusted,
        },
        McpTool {
            name: "cvg_checkpoint_save".to_string(),
            description: "Save a plan checkpoint for fault-tolerant state recovery.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "plan_id": {"type": "integer", "description": "Plan ID to checkpoint"}
                },
                "required": ["plan_id"]
            }),
            min_ring: Ring::Trusted,
        },
        // Agents — read (Ring 3: available to all callers including sandboxed)
        McpTool {
            name: "cvg_list_agents".to_string(),
            description: "List registered agents with their status and last heartbeat.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            min_ring: Ring::Sandboxed,
        },
        // Agents — write (Ring 1)
        McpTool {
            name: "cvg_agent_start".to_string(),
            description: "Register an agent as active. Call on session start.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Agent name, e.g. claude-macbookpro-12345"},
                    "task_id": {"type": "integer", "description": "Associated task ID (optional)"}
                },
                "required": ["name"]
            }),
            min_ring: Ring::Trusted,
        },
        McpTool {
            name: "cvg_agent_complete".to_string(),
            description: "Mark an agent as completed. Call before agent exits.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Agent name to deregister"}
                },
                "required": ["name"]
            }),
            min_ring: Ring::Trusted,
        },
        // Mesh — read (Ring 2)
        McpTool {
            name: "cvg_mesh_status".to_string(),
            description: "Get peer topology, mesh state, and active connections.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            min_ring: Ring::Community,
        },
        McpTool {
            name: "cvg_node_readiness".to_string(),
            description: "Run node health checks and return readiness report.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            min_ring: Ring::Community,
        },
        // Metrics — read (Ring 2)
        McpTool {
            name: "cvg_cost_summary".to_string(),
            description: "Get spending overview: total_cost, active_plans, total_plans.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            min_ring: Ring::Community,
        },
        // Kernel — read (Ring 2)
        McpTool {
            name: "cvg_kernel_status".to_string(),
            description: "Get kernel status: models loaded, uptime, active audio node.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            min_ring: Ring::Community,
        },
        // Kernel — ask (Ring 1)
        McpTool {
            name: "cvg_kernel_ask".to_string(),
            description: "Ask the local LLM (Qwen) a question with platform context injected.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "prompt": {"type": "string", "description": "Question or instruction for the local kernel LLM"}
                },
                "required": ["prompt"]
            }),
            min_ring: Ring::Trusted,
        },
        // Actions — notify (Ring 1)
        McpTool {
            name: "cvg_notify".to_string(),
            description: "Send a notification via Telegram or ntfy.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {"type": "string", "description": "Notification body"},
                    "title": {"type": "string", "description": "Notification title (optional)"},
                    "severity": {
                        "type": "string",
                        "enum": ["info", "warning", "error"],
                        "description": "Severity level, defaults to info"
                    }
                },
                "required": ["message"]
            }),
            min_ring: Ring::Trusted,
        },
        // Actions — restart node (Ring 0 only)
        McpTool {
            name: "cvg_restart_node".to_string(),
            description: "Trigger recovery for a target node. Core ring only.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target": {"type": "string", "description": "Target node hostname, e.g. macProM1"}
                },
                "required": ["target"]
            }),
            min_ring: Ring::Core,
        },
        McpTool {
            name: "cvg_assign_role".to_string(),
            description: "Assign a role to a mesh node (kernel, executor, coordinator).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "node": {"type": "string"},
                    "role": {"type": "string", "enum": ["kernel", "executor", "coordinator", "worker"]},
                    "capabilities": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["node", "role"]
            }),
            min_ring: Ring::Core,
        },
        McpTool {
            name: "cvg_interrupt_agent".to_string(),
            description: "Interrupt a blocked/stalled agent via IPC bus.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "agent_name": {"type": "string"},
                    "reason": {"type": "string"}
                },
                "required": ["agent_name", "reason"]
            }),
            min_ring: Ring::Trusted,
        },
        McpTool {
            name: "cvg_reschedule_task".to_string(),
            description: "Reschedule a task from one node to another.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": {"type": "integer"},
                    "from_node": {"type": "string"},
                    "to_node": {"type": "string"},
                    "reason": {"type": "string"}
                },
                "required": ["task_id", "to_node", "reason"]
            }),
            min_ring: Ring::Trusted,
        },
    ]
}
