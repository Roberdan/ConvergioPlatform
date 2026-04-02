// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Kernel tool functions — MCP-like intelligence layer calling daemon API.
// Called by engine.ask() when Mistral uses function calling.

use serde_json::{json, Value};
use std::time::Duration;

// ── Tool registry ────────────────────────────────────────────────────────────
/// Describes a tool for prompt injection into the LLM context.
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
}

/// Returns descriptions for all available tools.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "get_plans",
            description: "List all plans with id, name, status, tasks_done, tasks_total.",
        },
        ToolDef {
            name: "get_plan_detail",
            description: "Get full plan JSON with tasks and waves. Args: {\"plan_id\": <u32>}.",
        },
        ToolDef {
            name: "get_costs",
            description: "Return total token cost, active plan count, total plan count.",
        },
        ToolDef {
            name: "get_node_status",
            description: "Return node readiness checks array from /api/node/readiness.",
        },
        ToolDef {
            name: "get_kernel_status",
            description: "Return kernel status: models loaded, uptime, active node.",
        },
        ToolDef {
            name: "get_agents",
            description: "Return list of registered agents from /api/ipc/agents.",
        },
        ToolDef {
            name: "restart_node",
            description: "Trigger recovery for a target node. Args: {\"target\": \"<node>\"}.",
        },
        ToolDef {
            name: "get_health",
            description: "Return platform health from /api/health.",
        },
        ToolDef {
            name: "get_agent_history",
            description: "Return recent agent activity from /api/agents/history.",
        },
        ToolDef {
            name: "get_mesh_status",
            description: "Return mesh peer status from /api/heartbeat/status.",
        },
        ToolDef {
            name: "create_org",
            description: "Create a virtual organization from a mission. Args: {\"name\": \"<str>\", \"mission\": \"<str>\"}.",
        },
        ToolDef {
            name: "scan_repo",
            description: "Scan a repository and create an org to manage it. Args: {\"path\": \"<str>\"}.",
        },
    ]
}

/// Dispatch a tool call by name. Returns JSON string or None for unknown tool.
pub fn call_tool(name: &str, daemon_url: &str, args: &Value) -> Option<String> {
    match name {
        "get_plans" => Some(get_plans(daemon_url)),
        "get_plan_detail" => {
            let plan_id = args.get("plan_id").and_then(|v| v.as_u64())? as u32;
            Some(get_plan_detail(daemon_url, plan_id))
        }
        "get_costs" => Some(get_costs(daemon_url)),
        "get_node_status" => Some(get_node_status(daemon_url)),
        "get_kernel_status" => Some(get_kernel_status(daemon_url)),
        "get_agents" => Some(get_agents(daemon_url)),
        "restart_node" => {
            let target = args
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            Some(restart_node(daemon_url, target))
        }
        "get_health" => Some(get_health(daemon_url)),
        "get_agent_history" => Some(get_agent_history(daemon_url)),
        "get_mesh_status" => Some(get_mesh_status(daemon_url)),
        "create_org" => {
            let n = args.get("name").and_then(|v| v.as_str()).unwrap_or("project");
            let m = args.get("mission").and_then(|v| v.as_str()).unwrap_or("");
            Some(crate::kernel::voice_route_project::route_create_project(n, m, daemon_url))
        }
        "scan_repo" => {
            let p = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            Some(crate::kernel::voice_route_project::route_create_org_from(p, daemon_url))
        }
        _ => None,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────
fn make_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

fn fetch_json(url: &str) -> Option<Value> {
    let resp = make_client().get(url).send().map_err(|e| {
        tracing::warn!("kernel.tools: fetch {url}: {e}");
    }).ok()?;
    resp.json::<Value>().map_err(|e| {
        tracing::warn!("kernel.tools: parse {url}: {e}");
    }).ok()
}

fn error_json(msg: &str) -> String {
    json!({"error": msg}).to_string()
}

// ── Tool implementations ─────────────────────────────────────────────────────

/// GET /api/plan-db/list → [{id, name, status, tasks_done, tasks_total}]
pub fn get_plans(daemon_url: &str) -> String {
    let url = format!("{daemon_url}/api/plan-db/list");
    let Some(body) = fetch_json(&url) else {
        return error_json("daemon unreachable");
    };
    let plans = body
        .get("plans")
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter()
                .map(|p| {
                    json!({
                        "id":          p.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                        "name":        p.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                        "status":      p.get("status").and_then(|v| v.as_str()).unwrap_or(""),
                        "tasks_done":  p.get("tasks_done").and_then(|v| v.as_u64()).unwrap_or(0),
                        "tasks_total": p.get("tasks_total").and_then(|v| v.as_u64()).unwrap_or(0),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    serde_json::to_string(&plans).unwrap_or_else(|_| error_json("serialization error"))
}

/// GET /api/plan-db/json/{plan_id} → plan + tasks + waves
pub fn get_plan_detail(daemon_url: &str, plan_id: u32) -> String {
    fetch_endpoint(daemon_url, &format!("/api/plan-db/json/{plan_id}"))
}

/// GET /api/plan-db/list → {total_cost, active_plans, total_plans}
pub fn get_costs(daemon_url: &str) -> String {
    let url = format!("{daemon_url}/api/plan-db/list");
    let Some(body) = fetch_json(&url) else {
        return error_json("daemon unreachable");
    };
    let plans = body
        .get("plans")
        .and_then(|p| p.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[]);
    let total_cost: f64 = plans
        .iter()
        .filter_map(|p| p.get("total_cost").and_then(|v| v.as_f64()))
        .sum();
    let active_plans = plans
        .iter()
        .filter(|p| p.get("status").and_then(|s| s.as_str()) == Some("doing"))
        .count();
    let result = json!({
        "total_cost":   total_cost,
        "active_plans": active_plans,
        "total_plans":  plans.len(),
    });
    serde_json::to_string(&result).unwrap_or_else(|_| error_json("serialization error"))
}

/// GET /api/node/readiness → checks array
pub fn get_node_status(d: &str) -> String { fetch_endpoint(d, "/api/node/readiness") }
/// GET /api/kernel/status → models, uptime, node
pub fn get_kernel_status(d: &str) -> String { fetch_endpoint(d, "/api/kernel/status") }
/// GET /api/ipc/agents → agent list
pub fn get_agents(d: &str) -> String { fetch_endpoint(d, "/api/ipc/agents") }

/// Trigger recovery action for a target node.
pub fn restart_node(daemon_url: &str, target: &str) -> String {
    let url = format!("{daemon_url}/api/node/recover");
    let result = make_client()
        .post(&url)
        .json(&json!({"target": target, "action": "restart"}))
        .send();
    match result {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body = resp.json::<Value>().unwrap_or_else(|_| json!({}));
            json!({"status": status, "result": body}).to_string()
        }
        Err(e) => error_json(&format!("recovery request failed: {e}")),
    }
}

/// Simple GET → JSON string helper for endpoints without custom parsing.
fn fetch_endpoint(daemon_url: &str, path: &str) -> String {
    let url = format!("{daemon_url}{path}");
    match fetch_json(&url) {
        Some(v) => serde_json::to_string(&v).unwrap_or_else(|_| error_json("serialization error")),
        None => error_json(&format!("{path} unreachable")),
    }
}

/// GET /api/health → platform health summary
pub fn get_health(d: &str) -> String { fetch_endpoint(d, "/api/health") }
/// GET /api/agents/history?limit=10 → recent agent activity
pub fn get_agent_history(d: &str) -> String { fetch_endpoint(d, "/api/agents/history?limit=10") }
/// GET /api/heartbeat/status → mesh peer status
pub fn get_mesh_status(d: &str) -> String { fetch_endpoint(d, "/api/heartbeat/status") }

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
