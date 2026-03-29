// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// MCP tool catalogue: all 18 tool definitions with JSON Schema and ring requirements.

use serde_json::json;

use crate::capabilities::ring::Ring;
use crate::mcp_server::tools::McpTool;

/// Returns the full catalogue of MCP tools (unfiltered).
pub fn all_tools() -> Vec<McpTool> {
    let mut tools = Vec::with_capacity(18);
    tools.extend(plan_tools());
    tools.extend(agent_tools());
    tools.extend(mesh_tools());
    tools.extend(metrics_tools());
    tools.extend(kernel_tools());
    tools.extend(action_tools());
    tools.extend(control_tools());
    tools
}

fn plan_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "cvg_list_plans".into(),
            description: "List all plans with id, name, status, tasks_done, tasks_total.".into(),
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
            name: "cvg_get_plan".into(),
            description: "Get full plan JSON with tasks, waves, and progress.".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "plan_id": {"type": "integer", "description": "Plan ID"} },
                "required": ["plan_id"]
            }),
            min_ring: Ring::Community,
        },
        McpTool {
            name: "cvg_update_task".into(),
            description: "Update task status. Valid: pending->in_progress, in_progress->submitted, submitted->done.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": {"type": "integer", "description": "Task ID"},
                    "status": {"type": "string", "enum": ["in_progress", "submitted", "done", "blocked"]},
                    "summary": {"type": "string", "description": "Completion summary (required for done)"}
                },
                "required": ["task_id", "status"]
            }),
            min_ring: Ring::Trusted,
        },
        McpTool {
            name: "cvg_checkpoint_save".into(),
            description: "Save a plan checkpoint for fault-tolerant state recovery.".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "plan_id": {"type": "integer", "description": "Plan ID to checkpoint"} },
                "required": ["plan_id"]
            }),
            min_ring: Ring::Trusted,
        },
    ]
}

fn agent_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "cvg_list_agents".into(),
            description: "List registered agents with their status and last heartbeat.".into(),
            input_schema: json!({"type": "object", "properties": {}}),
            min_ring: Ring::Sandboxed,
        },
        McpTool {
            name: "cvg_agent_start".into(),
            description: "Register an agent as active. Call on session start.".into(),
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
            name: "cvg_agent_complete".into(),
            description: "Mark an agent as completed. Call before agent exits.".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "name": {"type": "string", "description": "Agent name to deregister"} },
                "required": ["name"]
            }),
            min_ring: Ring::Trusted,
        },
        McpTool {
            name: "cvg_invoke_agent".into(),
            description: "Invoke a Convergio agent by name. Reads definition, builds prompt, runs claude CLI.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "agent_name": {"type": "string", "description": "Agent name, e.g. dario-debugger"},
                    "task": {"type": "string", "description": "Task description or prompt for the agent"},
                    "context": {"type": "string", "description": "Additional context to inject (optional)"}
                },
                "required": ["agent_name", "task"]
            }),
            min_ring: Ring::Trusted,
        },
    ]
}

fn mesh_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "cvg_mesh_status".into(),
            description: "Get peer topology, mesh state, and active connections.".into(),
            input_schema: json!({"type": "object", "properties": {}}),
            min_ring: Ring::Community,
        },
        McpTool {
            name: "cvg_node_readiness".into(),
            description: "Run node health checks and return readiness report.".into(),
            input_schema: json!({"type": "object", "properties": {}}),
            min_ring: Ring::Community,
        },
    ]
}

fn metrics_tools() -> Vec<McpTool> {
    vec![McpTool {
        name: "cvg_cost_summary".into(),
        description: "Get spending overview: total_cost, active_plans, total_plans.".into(),
        input_schema: json!({"type": "object", "properties": {}}),
        min_ring: Ring::Community,
    }]
}

fn kernel_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "cvg_kernel_status".into(),
            description: "Get kernel status: models loaded, uptime, active audio node.".into(),
            input_schema: json!({"type": "object", "properties": {}}),
            min_ring: Ring::Community,
        },
        McpTool {
            name: "cvg_kernel_ask".into(),
            description: "Ask the local LLM (Qwen) a question with platform context injected.".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "prompt": {"type": "string", "description": "Question for the kernel LLM"} },
                "required": ["prompt"]
            }),
            min_ring: Ring::Trusted,
        },
    ]
}

fn action_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "cvg_notify".into(),
            description: "Send a notification via Telegram or ntfy.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {"type": "string", "description": "Notification body"},
                    "title": {"type": "string", "description": "Notification title (optional)"},
                    "severity": {"type": "string", "enum": ["info", "warning", "error"]}
                },
                "required": ["message"]
            }),
            min_ring: Ring::Trusted,
        },
        McpTool {
            name: "cvg_restart_node".into(),
            description: "Trigger recovery for a target node. Core ring only.".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "target": {"type": "string", "description": "Target node hostname"} },
                "required": ["target"]
            }),
            min_ring: Ring::Core,
        },
    ]
}

fn control_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "cvg_assign_role".into(),
            description: "Assign a role to a mesh node (kernel, executor, coordinator).".into(),
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
            name: "cvg_interrupt_agent".into(),
            description: "Interrupt a blocked/stalled agent via IPC bus.".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "agent_name": {"type": "string"}, "reason": {"type": "string"} },
                "required": ["agent_name", "reason"]
            }),
            min_ring: Ring::Trusted,
        },
        McpTool {
            name: "cvg_reschedule_task".into(),
            description: "Reschedule a task from one node to another.".into(),
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
