// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// MCP org tools: 7 org management + 3 digest tools + handlers.

use serde_json::{json, Value};
use std::time::Duration;

use crate::capabilities::ring::Ring;
use crate::mcp_server::security::McpError;
use crate::mcp_server::tools::McpTool;

pub fn org_tools() -> Vec<McpTool> {
    vec![
        McpTool { name: "cvg_org_create".into(),
            description: "Create a new org with mission, objectives, CEO agent, and budget.".into(),
            input_schema: json!({"type":"object","properties":{"name":{"type":"string"},"mission":{"type":"string"},"objectives":{"type":"string"},"ceo_agent":{"type":"string"},"budget":{"type":"number"}},"required":["name"]}),
            min_ring: Ring::Trusted },
        McpTool { name: "cvg_org_list".into(),
            description: "List all organisations registered in the platform.".into(),
            input_schema: json!({"type":"object","properties":{}}),
            min_ring: Ring::Community },
        McpTool { name: "cvg_org_show".into(),
            description: "Show details of a specific org by ID.".into(),
            input_schema: json!({"type":"object","properties":{"org_id":{"type":"string"}},"required":["org_id"]}),
            min_ring: Ring::Community },
        McpTool { name: "cvg_org_members".into(),
            description: "Add or update a member (agent) in an org with a role and department.".into(),
            input_schema: json!({"type":"object","properties":{"org_id":{"type":"string"},"agent":{"type":"string"},"role":{"type":"string"},"dept":{"type":"string"}},"required":["org_id","agent","role"]}),
            min_ring: Ring::Trusted },
        McpTool { name: "cvg_org_services".into(),
            description: "Register a service endpoint for an org.".into(),
            input_schema: json!({"type":"object","properties":{"org_id":{"type":"string"},"name":{"type":"string"},"endpoint":{"type":"string"},"description":{"type":"string"}},"required":["org_id","name","endpoint"]}),
            min_ring: Ring::Trusted },
        McpTool { name: "cvg_org_decide".into(),
            description: "Record a decision with rationale and author for an org.".into(),
            input_schema: json!({"type":"object","properties":{"org_id":{"type":"string"},"decision":{"type":"string"},"rationale":{"type":"string"},"made_by":{"type":"string"}},"required":["org_id","decision","rationale","made_by"]}),
            min_ring: Ring::Trusted },
        McpTool { name: "cvg_org_telemetry".into(),
            description: "Get telemetry metrics for an org (tasks, agents, costs).".into(),
            input_schema: json!({"type":"object","properties":{"org_id":{"type":"string"}},"required":["org_id"]}),
            min_ring: Ring::Community },
        McpTool { name: "cvg_org_digest".into(),
            description: "Get the latest digest for an org.".into(),
            input_schema: json!({"type":"object","properties":{"org_id":{"type":"string"}},"required":["org_id"]}),
            min_ring: Ring::Community },
        McpTool { name: "cvg_org_digest_generate".into(),
            description: "Generate a new digest for an org (triggers summarisation).".into(),
            input_schema: json!({"type":"object","properties":{"org_id":{"type":"string"}},"required":["org_id"]}),
            min_ring: Ring::Community },
        McpTool { name: "cvg_morning_brief".into(),
            description: "Get the morning brief: cross-org summary of plans, alerts, and priorities.".into(),
            input_schema: json!({"type":"object","properties":{}}),
            min_ring: Ring::Community },
    ]
}

pub fn handle_org_tool(
    name: &str,
    daemon_url: &str,
    token: Option<&str>,
    args: &Value,
) -> Result<Value, McpError> {
    match name {
        "cvg_org_create" => {
            let n = str_arg(args, "name")?;
            let mut body = json!({"name": n});
            for k in &["mission", "objectives", "ceo_agent"] {
                if let Some(v) = args.get(*k).and_then(|v| v.as_str()) {
                    body[k] = json!(v);
                }
            }
            if let Some(b) = args.get("budget").and_then(|v| v.as_f64()) {
                body["budget"] = json!(b);
            }
            post(daemon_url, token, "/api/orgs", &body)
        }
        "cvg_org_list" => get(daemon_url, token, "/api/orgs"),
        "cvg_org_show" => {
            let id = str_arg(args, "org_id")?;
            get(daemon_url, token, &format!("/api/orgs/{id}"))
        }
        "cvg_org_members" => {
            let id = str_arg(args, "org_id")?;
            let agent = str_arg(args, "agent")?;
            let role = str_arg(args, "role")?;
            let mut body = json!({"agent": agent, "role": role});
            if let Some(d) = args.get("dept").and_then(|v| v.as_str()) {
                body["dept"] = json!(d);
            }
            post(daemon_url, token, &format!("/api/orgs/{id}/members"), &body)
        }
        "cvg_org_services" => {
            let id = str_arg(args, "org_id")?;
            let name = str_arg(args, "name")?;
            let endpoint = str_arg(args, "endpoint")?;
            let mut body = json!({"name": name, "endpoint": endpoint});
            if let Some(d) = args.get("description").and_then(|v| v.as_str()) {
                body["description"] = json!(d);
            }
            post(daemon_url, token, &format!("/api/orgs/{id}/services"), &body)
        }
        "cvg_org_decide" => {
            let id = str_arg(args, "org_id")?;
            let decision = str_arg(args, "decision")?;
            let rationale = str_arg(args, "rationale")?;
            let made_by = str_arg(args, "made_by")?;
            post(daemon_url, token, &format!("/api/orgs/{id}/decisions"),
                &json!({"decision": decision, "rationale": rationale, "made_by": made_by}))
        }
        "cvg_org_telemetry" => {
            let id = str_arg(args, "org_id")?;
            get(daemon_url, token, &format!("/api/orgs/{id}/telemetry"))
        }
        "cvg_org_digest" => {
            let id = str_arg(args, "org_id")?;
            get(daemon_url, token, &format!("/api/orgs/{id}/digest"))
        }
        "cvg_org_digest_generate" => {
            let id = str_arg(args, "org_id")?;
            post(daemon_url, token, &format!("/api/orgs/{id}/digest/generate"), &json!({}))
        }
        "cvg_morning_brief" => get(daemon_url, token, "/api/digest/morning"),
        _ => Err(McpError::InvalidParams("unknown org tool name")),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn str_arg<'a>(args: &'a Value, key: &'static str) -> Result<&'a str, McpError> {
    args.get(key).and_then(|v| v.as_str()).ok_or(McpError::InvalidParams(key))
}

fn make_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

fn get(daemon_url: &str, token: Option<&str>, path: &str) -> Result<Value, McpError> {
    let mut req = make_client().get(format!("{daemon_url}{path}"));
    if let Some(t) = token { req = req.bearer_auth(t); }
    let resp = req.send().map_err(|_| McpError::DaemonUnreachable)?;
    if !resp.status().is_success() {
        return Err(McpError::DaemonError(format!("HTTP {}", resp.status().as_u16())));
    }
    resp.json::<Value>().map_err(|e| McpError::DaemonError(e.to_string()))
}

fn post(daemon_url: &str, token: Option<&str>, path: &str, body: &Value) -> Result<Value, McpError> {
    let mut req = make_client().post(format!("{daemon_url}{path}")).json(body);
    if let Some(t) = token { req = req.bearer_auth(t); }
    let resp = req.send().map_err(|_| McpError::DaemonUnreachable)?;
    if !resp.status().is_success() {
        return Err(McpError::DaemonError(format!("HTTP {}", resp.status().as_u16())));
    }
    resp.json::<Value>().map_err(|e| McpError::DaemonError(e.to_string()))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_schemas_validate() {
        let tools = org_tools();
        assert_eq!(tools.len(), 10, "expected exactly 10 org tools (7 org + 3 digest)");
        for tool in &tools {
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());
            assert!(tool.input_schema.is_object());
        }
    }

    #[test]
    fn ring_assignments() {
        let tools = org_tools();
        for name in &["cvg_org_create", "cvg_org_members", "cvg_org_services", "cvg_org_decide"] {
            let tool = tools.iter().find(|t| t.name == *name)
                .unwrap_or_else(|| panic!("{name} not found"));
            assert_eq!(tool.min_ring.as_u8(), Ring::Trusted.as_u8(), "{name} must be Trusted");
        }
        for name in &["cvg_org_list", "cvg_org_show", "cvg_org_telemetry",
                      "cvg_org_digest", "cvg_org_digest_generate", "cvg_morning_brief"] {
            let tool = tools.iter().find(|t| t.name == *name)
                .unwrap_or_else(|| panic!("{name} not found"));
            assert_eq!(tool.min_ring.as_u8(), Ring::Community.as_u8(), "{name} must be Community");
        }
    }
}
