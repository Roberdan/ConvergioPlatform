// Org domain tools — 10 tools for organization CRUD and intelligence.

use super::{ToolDef, ToolMethod, ToolParam, ToolTier};

const P_ORG_ID: &[ToolParam] = &[ToolParam {
    name: "org_id",
    param_type: "string",
    required: true,
}];

const P_ORG_CREATE: &[ToolParam] = &[
    ToolParam { name: "name", param_type: "string", required: true },
    ToolParam { name: "mission", param_type: "string", required: true },
    ToolParam { name: "objectives", param_type: "string", required: true },
    ToolParam { name: "ceo_agent", param_type: "string", required: true },
    ToolParam { name: "budget", param_type: "number", required: false },
];

const P_ORG_MEMBERS: &[ToolParam] = &[
    ToolParam { name: "org_id", param_type: "string", required: true },
    ToolParam { name: "agent", param_type: "string", required: true },
    ToolParam { name: "role", param_type: "string", required: true },
    ToolParam { name: "dept", param_type: "string", required: false },
];

const P_ORG_SERVICES: &[ToolParam] = &[
    ToolParam { name: "org_id", param_type: "string", required: true },
    ToolParam { name: "name", param_type: "string", required: true },
    ToolParam { name: "endpoint", param_type: "string", required: true },
    ToolParam { name: "description", param_type: "string", required: false },
];

const P_ORG_DECIDE: &[ToolParam] = &[
    ToolParam { name: "org_id", param_type: "string", required: true },
    ToolParam { name: "decision", param_type: "string", required: true },
    ToolParam { name: "rationale", param_type: "string", required: true },
    ToolParam { name: "made_by", param_type: "string", required: true },
];

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "org_create",
            description: "Create a new organization. Args: name, mission, objectives, ceo_agent, optional budget.",
            endpoint: "/api/orgs",
            method: ToolMethod::Post,
            params: P_ORG_CREATE,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "org_list",
            description: "List all organizations.",
            endpoint: "/api/orgs",
            method: ToolMethod::Get,
            params: &[],
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "org_show",
            description: "Show organization details. Args: org_id.",
            endpoint: "/api/orgs/{org_id}",
            method: ToolMethod::Get,
            params: P_ORG_ID,
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "org_add_member",
            description: "Add member to organization. Args: org_id, agent, role.",
            endpoint: "/api/orgs/{org_id}/members",
            method: ToolMethod::Post,
            params: P_ORG_MEMBERS,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "org_add_service",
            description: "Register a service for organization. Args: org_id, name, endpoint.",
            endpoint: "/api/orgs/{org_id}/services",
            method: ToolMethod::Post,
            params: P_ORG_SERVICES,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "org_decide",
            description: "Record a decision for organization. Args: org_id, decision, rationale, made_by.",
            endpoint: "/api/orgs/{org_id}/decisions",
            method: ToolMethod::Post,
            params: P_ORG_DECIDE,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "org_telemetry",
            description: "Get organization telemetry data. Args: org_id.",
            endpoint: "/api/orgs/{org_id}/telemetry",
            method: ToolMethod::Get,
            params: P_ORG_ID,
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "org_digest",
            description: "Get organization digest summary. Args: org_id.",
            endpoint: "/api/orgs/{org_id}/digest",
            method: ToolMethod::Get,
            params: P_ORG_ID,
            tier: ToolTier::Read,
        },
        ToolDef {
            name: "org_digest_generate",
            description: "Generate fresh org digest. Args: org_id.",
            endpoint: "/api/orgs/{org_id}/digest/generate",
            method: ToolMethod::Post,
            params: P_ORG_ID,
            tier: ToolTier::Write,
        },
        ToolDef {
            name: "morning_brief",
            description: "Get the daily morning brief across all orgs.",
            endpoint: "/api/digest/morning",
            method: ToolMethod::Get,
            params: &[],
            tier: ToolTier::Read,
        },
    ]
}
