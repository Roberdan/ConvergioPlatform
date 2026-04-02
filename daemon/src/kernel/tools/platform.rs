// Platform tools — 12 tools for plan lifecycle, validation, memory, budget.

use super::{ToolDef, ToolMethod, ToolParam, ToolTier};

const P_CREATE_PLAN: &[ToolParam] = &[
    ToolParam { name: "name", param_type: "string", required: true },
    ToolParam { name: "project_id", param_type: "string", required: true },
];

const P_PLAN_ID: &[ToolParam] = &[ToolParam {
    name: "plan_id",
    param_type: "integer",
    required: true,
}];

const P_CREATE_TASK: &[ToolParam] = &[
    ToolParam { name: "plan_id", param_type: "integer", required: true },
    ToolParam { name: "wave_id_fk", param_type: "integer", required: true },
    ToolParam { name: "task_id", param_type: "string", required: true },
    ToolParam { name: "title", param_type: "string", required: true },
];

const P_VALIDATION: &[ToolParam] = &[
    ToolParam { name: "task_id", param_type: "integer", required: true },
    ToolParam { name: "verdict", param_type: "string", required: true },
    ToolParam { name: "validator", param_type: "string", required: true },
];

const P_WORKSPACE: &[ToolParam] = &[ToolParam {
    name: "workspace",
    param_type: "string",
    required: true,
}];

const P_REMEMBER: &[ToolParam] = &[
    ToolParam { name: "key", param_type: "string", required: true },
    ToolParam { name: "value", param_type: "string", required: true },
    ToolParam { name: "agent", param_type: "string", required: true },
];

const P_RECALL: &[ToolParam] = &[
    ToolParam { name: "key", param_type: "string", required: true },
    ToolParam { name: "agent", param_type: "string", required: true },
];

const P_MESSAGES: &[ToolParam] = &[
    ToolParam { name: "agent", param_type: "string", required: true },
    ToolParam { name: "limit", param_type: "integer", required: false },
];

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "create_plan",
            description: "Create a new plan. Args: name, project_id.",
            endpoint: "/api/plan-db/create",
            method: ToolMethod::Post,
            params: P_CREATE_PLAN,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "start_plan",
            description: "Start a plan. Args: plan_id.",
            endpoint: "/api/plan-db/start/{plan_id}",
            method: ToolMethod::Post,
            params: P_PLAN_ID,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "create_task",
            description: "Create task in a plan wave. Args: plan_id, wave_id_fk, task_id, title.",
            endpoint: "/api/plan-db/task/create",
            method: ToolMethod::Post,
            params: P_CREATE_TASK,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "record_validation",
            description: "Record validation verdict. Args: task_id, verdict, validator.",
            endpoint: "/api/validation/record",
            method: ToolMethod::Post,
            params: P_VALIDATION,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "quality_gate",
            description: "Run quality gate on workspace. Args: workspace.",
            endpoint: "/api/workspace/quality-gate",
            method: ToolMethod::Post,
            params: P_WORKSPACE,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "health_deep",
            description: "Deep health check of the platform.",
            endpoint: "/api/health/deep",
            method: ToolMethod::Get,
            params: &[],
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "list_workspaces",
            description: "List all workspaces.",
            endpoint: "/api/workspace/list",
            method: ToolMethod::Get,
            params: &[],
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "remember",
            description: "Store a key-value pair in agent memory. Args: key, value, agent.",
            endpoint: "/api/memory/remember",
            method: ToolMethod::Post,
            params: P_REMEMBER,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "recall",
            description: "Recall a value from agent memory. Args: key, agent.",
            endpoint: "/api/memory/recall?key={key}&agent={agent}",
            method: ToolMethod::Get,
            params: P_RECALL,
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "budget",
            description: "Get budget status and spending.",
            endpoint: "/api/budget/status",
            method: ToolMethod::Get,
            params: &[],
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "agent_catalog",
            description: "Get the full agent catalog with capabilities.",
            endpoint: "/api/agents/catalog",
            method: ToolMethod::Get,
            params: &[],
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "list_messages",
            description: "List IPC messages for an agent. Args: agent, optional limit.",
            endpoint: "/api/ipc/messages?agent={agent}&limit={limit}",
            method: ToolMethod::Get,
            params: P_MESSAGES,
            tier: ToolTier::Read,
        },
    ]
}
