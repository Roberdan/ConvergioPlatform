// Agent and chat tools — 7 tools for agent lifecycle and IPC.

use super::{ToolDef, ToolMethod, ToolParam, ToolTier};

const P_AGENT_SEND: &[ToolParam] = &[
    ToolParam { name: "from", param_type: "string", required: true },
    ToolParam { name: "to", param_type: "string", required: true },
    ToolParam { name: "content", param_type: "string", required: true },
];

const P_AGENT_ASK: &[ToolParam] = &[
    ToolParam { name: "from", param_type: "string", required: true },
    ToolParam { name: "to", param_type: "string", required: true },
    ToolParam { name: "message", param_type: "string", required: true },
    ToolParam { name: "timeout_secs", param_type: "integer", required: false },
];

const P_AGENT_START: &[ToolParam] = &[
    ToolParam { name: "agent_id", param_type: "string", required: true },
    ToolParam { name: "plan_id", param_type: "integer", required: true },
];

const P_AGENT_COMPLETE: &[ToolParam] = &[ToolParam {
    name: "agent_id",
    param_type: "string",
    required: true,
}];

const P_INVOKE: &[ToolParam] = &[
    ToolParam { name: "from", param_type: "string", required: true },
    ToolParam { name: "to", param_type: "string", required: true },
    ToolParam { name: "message", param_type: "string", required: true },
];

const P_CREATE_AGENT: &[ToolParam] = &[
    ToolParam { name: "name", param_type: "string", required: true },
    ToolParam { name: "category", param_type: "string", required: false },
    ToolParam { name: "description", param_type: "string", required: false },
    ToolParam { name: "model", param_type: "string", required: false },
];

const P_MESSAGES: &[ToolParam] = &[
    ToolParam { name: "to_agent", param_type: "string", required: true },
    ToolParam { name: "channel", param_type: "string", required: false },
    ToolParam { name: "limit", param_type: "integer", required: false },
];

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "list_agents",
            description: "List all registered agents.",
            endpoint: "/api/ipc/agents",
            method: ToolMethod::Get,
            params: &[],
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "agent_send",
            description: "Send direct message to an agent. Args: from, to, content.",
            endpoint: "/api/ipc/send-direct",
            method: ToolMethod::Post,
            params: P_AGENT_SEND,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "agent_ask",
            description: "Ask an agent and wait for reply. Args: from, to, message, optional timeout_secs.",
            endpoint: "/api/ipc/ask",
            method: ToolMethod::Post,
            params: P_AGENT_ASK,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "agent_start",
            description: "Register agent start for a plan. Args: agent_id, plan_id.",
            endpoint: "/api/plan-db/agent/start",
            method: ToolMethod::Post,
            params: P_AGENT_START,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "agent_complete",
            description: "Register agent completion. Args: agent_id.",
            endpoint: "/api/plan-db/agent/complete",
            method: ToolMethod::Post,
            params: P_AGENT_COMPLETE,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "invoke_agent",
            description: "Invoke a named agent with a task. Args: from (kernel identity), to (agent name), message (task).",
            endpoint: "/api/ipc/ask",
            method: ToolMethod::Post,
            params: P_INVOKE,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "create_agent",
            description: "Create a new agent. Args: name, optional category/description/model.",
            endpoint: "/api/agents/create",
            method: ToolMethod::Post,
            params: P_CREATE_AGENT,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "list_messages",
            description: "List IPC messages for an agent. Args: to_agent, optional channel (filter by channel name), optional limit.",
            endpoint: "/api/ipc/messages?to_agent={to_agent}&channel={channel}&limit={limit}",
            method: ToolMethod::Get,
            params: P_MESSAGES,
            tier: ToolTier::Read,
        },
    ]
}
