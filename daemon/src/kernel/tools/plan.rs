// Plan domain tools — 4 tools for plan/task/checkpoint operations.

use super::{ToolDef, ToolMethod, ToolParam, ToolTier};

const P_PLAN_ID: &[ToolParam] = &[ToolParam {
    name: "plan_id",
    param_type: "integer",
    required: true,
}];

const P_UPDATE_TASK: &[ToolParam] = &[
    ToolParam { name: "task_id", param_type: "integer", required: true },
    ToolParam { name: "status", param_type: "string", required: true },
    ToolParam { name: "summary", param_type: "string", required: false },
];

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "get_plans",
            description: "List all plans with id, name, status, tasks_done, tasks_total.",
            endpoint: "/api/plan-db/list",
            method: ToolMethod::Get,
            params: &[],
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "get_plan_detail",
            description: "Get full plan JSON with tasks and waves. Args: plan_id.",
            endpoint: "/api/plan-db/json/{plan_id}",
            method: ToolMethod::Get,
            params: P_PLAN_ID,
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "update_task",
            description: "Update task status. Args: task_id, status, optional summary.",
            endpoint: "/api/plan-db/task/update",
            method: ToolMethod::Post,
            params: P_UPDATE_TASK,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "checkpoint_save",
            description: "Save a checkpoint for a plan. Args: plan_id.",
            endpoint: "/api/plan-db/checkpoint/save",
            method: ToolMethod::Post,
            params: P_PLAN_ID,
            tier: ToolTier::Write,
        },
    ]
}
