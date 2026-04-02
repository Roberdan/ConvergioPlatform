// Infra tools — 10 tools for mesh, node, kernel, notifications, control.

use super::{ToolDef, ToolMethod, ToolParam, ToolTier};

const P_PROMPT: &[ToolParam] = &[ToolParam {
    name: "prompt",
    param_type: "string",
    required: true,
}];

const P_MESSAGE: &[ToolParam] = &[
    ToolParam { name: "message", param_type: "string", required: true },
    ToolParam { name: "title", param_type: "string", required: false },
    ToolParam { name: "severity", param_type: "string", required: false },
];

const P_TARGET: &[ToolParam] = &[ToolParam {
    name: "target",
    param_type: "string",
    required: true,
}];

const P_ASSIGN_ROLE: &[ToolParam] = &[
    ToolParam { name: "node", param_type: "string", required: true },
    ToolParam { name: "role", param_type: "string", required: true },
];

const P_INTERRUPT: &[ToolParam] = &[
    ToolParam { name: "agent_name", param_type: "string", required: true },
    ToolParam { name: "reason", param_type: "string", required: true },
];

const P_RESCHEDULE: &[ToolParam] = &[
    ToolParam { name: "task_id", param_type: "integer", required: true },
    ToolParam { name: "to_node", param_type: "string", required: true },
    ToolParam { name: "reason", param_type: "string", required: true },
];

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "mesh_status",
            description: "Get mesh peer status and connectivity.",
            endpoint: "/api/mesh",
            method: ToolMethod::Get,
            params: &[],
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "node_readiness",
            description: "Get node readiness checks.",
            endpoint: "/api/node/readiness",
            method: ToolMethod::Get,
            params: &[],
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "cost_summary",
            description: "Get cost summary across all plans.",
            endpoint: "/api/plan-db/list",
            method: ToolMethod::Get,
            params: &[],
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "kernel_status",
            description: "Get kernel status: models loaded, uptime, active node.",
            endpoint: "/api/kernel/status",
            method: ToolMethod::Get,
            params: &[],
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "kernel_ask",
            description: "Ask the kernel a question (routes to local or cloud). Args: prompt.",
            endpoint: "/api/kernel/ask",
            method: ToolMethod::Post,
            params: P_PROMPT,
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "notify",
            description: "Send a notification. Args: message, optional title, severity.",
            endpoint: "/api/notify",
            method: ToolMethod::Post,
            params: P_MESSAGE,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "restart_node",
            description: "Trigger node recovery/restart. Args: target.",
            endpoint: "/api/node/recover",
            method: ToolMethod::Post,
            params: P_TARGET,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "assign_role",
            description: "Assign role to a mesh node. Args: node, role.",
            endpoint: "/api/node/assign-role",
            method: ToolMethod::Post,
            params: P_ASSIGN_ROLE,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "interrupt_agent",
            description: "Interrupt a running agent. Args: agent_name, reason.",
            endpoint: "/api/agent/interrupt",
            method: ToolMethod::Post,
            params: P_INTERRUPT,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "reschedule_task",
            description: "Reschedule task to another node. Args: task_id, to_node, reason.",
            endpoint: "/api/task/reschedule",
            method: ToolMethod::Post,
            params: P_RESCHEDULE,
            tier: ToolTier::Write,
        },
    ]
}
