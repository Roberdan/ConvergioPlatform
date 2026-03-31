use serde_json::json;

use crate::capabilities::ring::Ring;
use crate::mcp_server::tools::McpTool;

pub fn plan_tools() -> Vec<McpTool> {
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
