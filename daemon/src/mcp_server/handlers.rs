// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// HTTP bridge: each MCP tool handler calls the daemon API at daemon_url.
// Pattern mirrors daemon/src/kernel/tools.rs — reqwest::blocking, 5-second timeout.

use serde_json::{json, Value};
use std::time::Duration;

use crate::mcp_server::security::McpError;

// ── HTTP helpers ──────────────────────────────────────────────────────────────

fn make_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

fn http_get(url: &str, token: Option<&str>) -> Result<Value, McpError> {
    let client = make_client();
    let mut req = client.get(url);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req.send().map_err(|_| McpError::DaemonUnreachable)?;
    if !resp.status().is_success() {
        return Err(McpError::DaemonError(format!("HTTP {}", resp.status().as_u16())));
    }
    resp.json::<Value>().map_err(|e| McpError::DaemonError(e.to_string()))
}

fn http_post(url: &str, token: Option<&str>, body: &Value) -> Result<Value, McpError> {
    let client = make_client();
    let mut req = client.post(url).json(body);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req.send().map_err(|_| McpError::DaemonUnreachable)?;
    if !resp.status().is_success() {
        return Err(McpError::DaemonError(format!("HTTP {}", resp.status().as_u16())));
    }
    resp.json::<Value>().map_err(|e| McpError::DaemonError(e.to_string()))
}

// ── Tool handlers ─────────────────────────────────────────────────────────────

/// Dispatch a tool call by name. Returns JSON result or McpError.
pub fn handle_tool_call(
    name: &str,
    args: &Value,
    daemon_url: &str,
    token: Option<&str>,
) -> Result<Value, McpError> {
    match name {
        "cvg_list_plans" => list_plans(daemon_url, token, args),
        "cvg_get_plan" => get_plan(daemon_url, token, args),
        "cvg_update_task" => update_task(daemon_url, token, args),
        "cvg_checkpoint_save" => checkpoint_save(daemon_url, token, args),
        "cvg_list_agents" => list_agents(daemon_url, token),
        "cvg_agent_start" => agent_start(daemon_url, token, args),
        "cvg_agent_complete" => agent_complete(daemon_url, token, args),
        "cvg_mesh_status" => mesh_status(daemon_url, token),
        "cvg_node_readiness" => node_readiness(daemon_url, token),
        "cvg_cost_summary" => cost_summary(daemon_url, token),
        "cvg_kernel_status" => kernel_status(daemon_url, token),
        "cvg_kernel_ask" => kernel_ask(daemon_url, token, args),
        "cvg_notify" => notify(daemon_url, token, args),
        "cvg_restart_node" => restart_node(daemon_url, token, args),
        "cvg_assign_role" => assign_role(daemon_url, token, args),
        "cvg_interrupt_agent" => interrupt_agent(daemon_url, token, args),
        "cvg_invoke_agent" => crate::mcp_server::invoke_agent::invoke_agent(args),
        "cvg_reschedule_task" => reschedule_task(daemon_url, token, args),
        _ => Err(McpError::InvalidParams("unknown tool name")),
    }
}

// ── Plans ─────────────────────────────────────────────────────────────────────

fn list_plans(daemon_url: &str, token: Option<&str>, args: &Value) -> Result<Value, McpError> {
    let url = format!("{daemon_url}/api/plan-db/list");
    let body = http_get(&url, token)?;
    let plans = body.get("plans").cloned().unwrap_or(json!([]));
    // Optional status_filter
    if let Some(filter) = args.get("status_filter").and_then(|v| v.as_str()) {
        let filtered: Vec<Value> = plans
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter(|p| p.get("status").and_then(|s| s.as_str()) == Some(filter))
            .cloned()
            .collect();
        return Ok(json!(filtered));
    }
    Ok(plans)
}

fn get_plan(daemon_url: &str, token: Option<&str>, args: &Value) -> Result<Value, McpError> {
    let plan_id = args
        .get("plan_id")
        .and_then(|v| v.as_i64())
        .ok_or(McpError::InvalidParams("plan_id is required"))?;
    let url = format!("{daemon_url}/api/plan-db/json/{plan_id}");
    http_get(&url, token)
}

fn update_task(daemon_url: &str, token: Option<&str>, args: &Value) -> Result<Value, McpError> {
    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_i64())
        .ok_or(McpError::InvalidParams("task_id is required"))?;
    let status = args
        .get("status")
        .and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("status is required"))?;
    let url = format!("{daemon_url}/api/plan-db/task/update");
    let payload = json!({
        "task_id": task_id,
        "status": status,
        "summary": args.get("summary").and_then(|v| v.as_str()).unwrap_or("")
    });
    http_post(&url, token, &payload)
}

fn checkpoint_save(daemon_url: &str, token: Option<&str>, args: &Value) -> Result<Value, McpError> {
    let plan_id = args
        .get("plan_id")
        .and_then(|v| v.as_i64())
        .ok_or(McpError::InvalidParams("plan_id is required"))?;
    let url = format!("{daemon_url}/api/plan-db/checkpoint/save");
    http_post(&url, token, &json!({"plan_id": plan_id}))
}

// ── Agents ────────────────────────────────────────────────────────────────────

fn list_agents(daemon_url: &str, token: Option<&str>) -> Result<Value, McpError> {
    let url = format!("{daemon_url}/api/ipc/agents");
    http_get(&url, token)
}

fn agent_start(daemon_url: &str, token: Option<&str>, args: &Value) -> Result<Value, McpError> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("name is required"))?;
    let url = format!("{daemon_url}/api/plan-db/agent/start");
    let mut payload = json!({"name": name});
    if let Some(task_id) = args.get("task_id").and_then(|v| v.as_i64()) {
        payload["task_id"] = json!(task_id);
    }
    http_post(&url, token, &payload)
}

fn agent_complete(daemon_url: &str, token: Option<&str>, args: &Value) -> Result<Value, McpError> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("name is required"))?;
    let url = format!("{daemon_url}/api/plan-db/agent/complete");
    http_post(&url, token, &json!({"name": name}))
}

// ── Mesh ──────────────────────────────────────────────────────────────────────

fn mesh_status(daemon_url: &str, token: Option<&str>) -> Result<Value, McpError> {
    let url = format!("{daemon_url}/api/mesh");
    http_get(&url, token)
}

fn node_readiness(daemon_url: &str, token: Option<&str>) -> Result<Value, McpError> {
    let url = format!("{daemon_url}/api/node/readiness");
    http_get(&url, token)
}

// ── Metrics ───────────────────────────────────────────────────────────────────

fn cost_summary(daemon_url: &str, token: Option<&str>) -> Result<Value, McpError> {
    let url = format!("{daemon_url}/api/plan-db/list");
    let body = http_get(&url, token)?;
    let plans = body
        .get("plans")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();
    let total_cost: f64 = plans
        .iter()
        .filter_map(|p| p.get("total_cost").and_then(|v| v.as_f64()))
        .sum();
    let active = plans
        .iter()
        .filter(|p| p.get("status").and_then(|s| s.as_str()) == Some("doing"))
        .count();
    Ok(json!({
        "total_cost": total_cost,
        "active_plans": active,
        "total_plans": plans.len(),
    }))
}

// ── Kernel ────────────────────────────────────────────────────────────────────

fn kernel_status(daemon_url: &str, token: Option<&str>) -> Result<Value, McpError> {
    let url = format!("{daemon_url}/api/kernel/status");
    http_get(&url, token)
}

fn kernel_ask(daemon_url: &str, token: Option<&str>, args: &Value) -> Result<Value, McpError> {
    let prompt = args
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("prompt is required"))?;
    let url = format!("{daemon_url}/api/kernel/ask");
    http_post(&url, token, &json!({"prompt": prompt}))
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn notify(daemon_url: &str, token: Option<&str>, args: &Value) -> Result<Value, McpError> {
    let message = args
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("message is required"))?;
    let url = format!("{daemon_url}/api/notify");
    let payload = json!({
        "message": message,
        "title": args.get("title").and_then(|v| v.as_str()).unwrap_or("Convergio"),
        "severity": args.get("severity").and_then(|v| v.as_str()).unwrap_or("info"),
    });
    http_post(&url, token, &payload)
}

fn restart_node(daemon_url: &str, token: Option<&str>, args: &Value) -> Result<Value, McpError> {
    let target = args.get("target").and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("target is required"))?;
    http_post(&format!("{daemon_url}/api/node/recover"), token, &json!({"target": target}))
}

fn assign_role(daemon_url: &str, token: Option<&str>, args: &Value) -> Result<Value, McpError> {
    http_post(&format!("{daemon_url}/api/node/assign-role"), token, args)
}

fn interrupt_agent(daemon_url: &str, token: Option<&str>, args: &Value) -> Result<Value, McpError> {
    http_post(&format!("{daemon_url}/api/agent/interrupt"), token, args)
}

fn reschedule_task(daemon_url: &str, token: Option<&str>, args: &Value) -> Result<Value, McpError> {
    http_post(&format!("{daemon_url}/api/task/reschedule"), token, args)
}
