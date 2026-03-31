// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// MCP platform tools: 12 tool definitions + handlers for pre-existing daemon APIs.

use serde_json::{json, Value};
use std::time::Duration;

use crate::capabilities::ring::Ring;
use crate::mcp_server::security::McpError;
use crate::mcp_server::tools::McpTool;

pub fn platform_tools() -> Vec<McpTool> {
    vec![
        McpTool { name: "cvg_create_plan".into(), description: "Create a new plan. Returns created plan id.".into(),
            input_schema: json!({"type":"object","properties":{"name":{"type":"string"},"project_id":{"type":"integer"}},"required":["name","project_id"]}),
            min_ring: Ring::Trusted },
        McpTool { name: "cvg_start_plan".into(), description: "Start a plan, transitioning it to active.".into(),
            input_schema: json!({"type":"object","properties":{"plan_id":{"type":"integer"}},"required":["plan_id"]}),
            min_ring: Ring::Trusted },
        McpTool { name: "cvg_create_task".into(), description: "Create a task within a plan wave.".into(),
            input_schema: json!({"type":"object","properties":{"plan_id":{"type":"integer"},"wave_id_fk":{"type":"integer"},"task_id":{"type":"string"},"title":{"type":"string"}},"required":["plan_id","wave_id_fk","task_id","title"]}),
            min_ring: Ring::Trusted },
        McpTool { name: "cvg_record_validation".into(), description: "Record validation verdict for a task (required before done).".into(),
            input_schema: json!({"type":"object","properties":{"task_id":{"type":"integer"},"verdict":{"type":"string","enum":["pass","fail"]},"validator":{"type":"string"}},"required":["task_id","verdict","validator"]}),
            min_ring: Ring::Trusted },
        McpTool { name: "cvg_quality_gate".into(), description: "Run quality gate checks on a workspace.".into(),
            input_schema: json!({"type":"object","properties":{"workspace":{"type":"string"}},"required":["workspace"]}),
            min_ring: Ring::Trusted },
        McpTool { name: "cvg_health_deep".into(), description: "Deep health check: DB, mesh, kernel, all subsystems.".into(),
            input_schema: json!({"type":"object","properties":{}}), min_ring: Ring::Community },
        McpTool { name: "cvg_list_workspaces".into(), description: "List registered workspaces with status and worktree info.".into(),
            input_schema: json!({"type":"object","properties":{}}), min_ring: Ring::Community },
        McpTool { name: "cvg_remember".into(), description: "Store a key-value memory entry for an agent.".into(),
            input_schema: json!({"type":"object","properties":{"key":{"type":"string"},"value":{"type":"string"},"agent":{"type":"string"}},"required":["key","value","agent"]}),
            min_ring: Ring::Community },
        McpTool { name: "cvg_recall".into(), description: "Recall a stored memory entry by key for an agent.".into(),
            input_schema: json!({"type":"object","properties":{"key":{"type":"string"},"agent":{"type":"string"}},"required":["key","agent"]}),
            min_ring: Ring::Community },
        McpTool { name: "cvg_budget".into(), description: "Get current token and cost budget status.".into(),
            input_schema: json!({"type":"object","properties":{}}), min_ring: Ring::Community },
        McpTool { name: "cvg_agent_catalog".into(), description: "List all available agent definitions from the catalog.".into(),
            input_schema: json!({"type":"object","properties":{}}), min_ring: Ring::Community },
        McpTool { name: "cvg_list_messages".into(), description: "List IPC messages for an agent, newest first.".into(),
            input_schema: json!({"type":"object","properties":{"agent":{"type":"string"},"limit":{"type":"integer"}},"required":["agent"]}),
            min_ring: Ring::Community },
    ]
}

pub fn handle_platform_tool(
    name: &str,
    daemon_url: &str,
    token: Option<&str>,
    args: &Value,
) -> Result<Value, McpError> {
    match name {
        "cvg_create_plan" => {
            let n = str_arg(args, "name")?; let pid = int_arg(args, "project_id")?;
            post(daemon_url, token, "/api/plan-db/create", &json!({"name":n,"project_id":pid}))
        }
        "cvg_start_plan" => {
            let id = int_arg(args, "plan_id")?;
            post(daemon_url, token, &format!("/api/plan-db/start/{id}"), &json!({}))
        }
        "cvg_create_task" => {
            let plan_id = int_arg(args, "plan_id")?; let wave = int_arg(args, "wave_id_fk")?;
            let tid = str_arg(args, "task_id")?; let title = str_arg(args, "title")?;
            post(daemon_url, token, "/api/plan-db/task/create",
                &json!({"plan_id":plan_id,"wave_id_fk":wave,"task_id":tid,"title":title}))
        }
        "cvg_record_validation" => {
            let task_id = int_arg(args, "task_id")?;
            let verdict = str_arg(args, "verdict")?; let validator = str_arg(args, "validator")?;
            post(daemon_url, token, "/api/validation/record",
                &json!({"task_id":task_id,"verdict":verdict,"validator":validator}))
        }
        "cvg_quality_gate" => {
            let ws = str_arg(args, "workspace")?;
            post(daemon_url, token, "/api/workspace/quality-gate", &json!({"workspace":ws}))
        }
        "cvg_health_deep" => get(daemon_url, token, "/api/health/deep"),
        "cvg_list_workspaces" => get(daemon_url, token, "/api/workspace/list"),
        "cvg_budget" => get(daemon_url, token, "/api/budget/status"),
        "cvg_agent_catalog" => get(daemon_url, token, "/api/agents/catalog"),
        "cvg_remember" => {
            let key = str_arg(args, "key")?; let val = str_arg(args, "value")?;
            let agent = str_arg(args, "agent")?;
            post(daemon_url, token, "/api/memory/remember", &json!({"key":key,"value":val,"agent":agent}))
        }
        "cvg_recall" => {
            let key = str_arg(args, "key")?; let agent = str_arg(args, "agent")?;
            get(daemon_url, token, &format!("/api/memory/recall?key={key}&agent={agent}"))
        }
        "cvg_list_messages" => {
            let agent = str_arg(args, "agent")?;
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
            get(daemon_url, token, &format!("/api/ipc/messages?agent={agent}&limit={limit}"))
        }
        _ => Err(McpError::InvalidParams("unknown platform tool name")),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn str_arg<'a>(args: &'a Value, key: &'static str) -> Result<&'a str, McpError> {
    args.get(key).and_then(|v| v.as_str()).ok_or(McpError::InvalidParams(key))
}

fn int_arg(args: &Value, key: &'static str) -> Result<i64, McpError> {
    args.get(key).and_then(|v| v.as_i64()).ok_or(McpError::InvalidParams(key))
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
        let tools = platform_tools();
        assert_eq!(tools.len(), 12, "expected exactly 12 platform tools");
        for tool in &tools {
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());
            assert!(tool.input_schema.is_object());
        }
    }

    #[test]
    fn ring_assignments() {
        let tools = platform_tools();
        for name in &["cvg_create_plan","cvg_start_plan","cvg_create_task","cvg_record_validation","cvg_quality_gate"] {
            let tool = tools.iter().find(|t| t.name == *name).unwrap_or_else(|| panic!("{name} not found"));
            assert_eq!(tool.min_ring.as_u8(), Ring::Trusted.as_u8(), "{name} must be Trusted");
        }
        for name in &["cvg_health_deep","cvg_list_workspaces","cvg_remember","cvg_recall","cvg_budget","cvg_agent_catalog","cvg_list_messages"] {
            let tool = tools.iter().find(|t| t.name == *name).unwrap_or_else(|| panic!("{name} not found"));
            assert_eq!(tool.min_ring.as_u8(), Ring::Community.as_u8(), "{name} must be Community");
        }
    }
}
