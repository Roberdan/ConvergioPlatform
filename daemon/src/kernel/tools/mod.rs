// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Declarative tool catalog for the kernel — all 43+ MCP tools.
// Each tool: name, description, HTTP endpoint, method, params, tier.

use serde_json::Value;
use std::time::Duration;
use tracing::warn;

pub mod agent;
pub mod infra;
pub mod org;
pub mod plan;
pub mod platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolTier {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy)]
pub enum ToolMethod {
    Get,
    Post,
}

pub struct ToolParam {
    pub name: &'static str,
    pub param_type: &'static str,
    pub required: bool,
}

pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub endpoint: &'static str,
    pub method: ToolMethod,
    pub params: &'static [ToolParam],
    pub tier: ToolTier,
}

pub struct ToolCatalog {
    pub tools: Vec<ToolDef>,
}

impl ToolCatalog {
    pub fn all() -> Self {
        let mut tools = Vec::with_capacity(50);
        tools.extend(plan::tools());
        tools.extend(org::tools());
        tools.extend(agent::tools());
        tools.extend(platform::tools());
        tools.extend(infra::tools());
        Self { tools }
    }

    pub fn find(&self, name: &str) -> Option<&ToolDef> {
        self.tools.iter().find(|t| t.name == name)
    }

    pub fn descriptions_block(&self) -> String {
        let mut block = String::from(
            "Strumenti disponibili. Per usarli rispondi con:\n\
             <tool_call>{\"name\":\"tool_name\",\"arguments\":{...}}</tool_call>\n\n",
        );
        for t in &self.tools {
            block += &format!("- {}: {}\n", t.name, t.description);
        }
        block += "\nUsa gli strumenti SOLO quando servono dati che non hai.\n";
        block
    }

    pub fn read_only_names(&self) -> Vec<&str> {
        self.tools
            .iter()
            .filter(|t| t.tier == ToolTier::Read)
            .map(|t| t.name)
            .collect()
    }

    pub fn call_tool(&self, name: &str, daemon_url: &str, args: &Value) -> Option<String> {
        let tool = self.find(name)?;
        let endpoint = match resolve_endpoint(tool.endpoint, args) {
            Ok(ep) => ep,
            Err(e) => return Some(format!("{{\"error\":\"{e}\"}}")),
        };
        let base = if daemon_url.is_empty() { self::daemon_url() } else { daemon_url.to_string() };
        let url = format!("{base}{endpoint}");
        match tool.method {
            ToolMethod::Get => http_get(&url),
            ToolMethod::Post => http_post(&url, args),
        }
    }

    /// Catalog with only Read-tier tools (for local inference safety).
    pub fn read_only() -> Self {
        let mut cat = Self::all();
        cat.tools.retain(|t| t.tier == ToolTier::Read);
        cat
    }
}

fn resolve_endpoint(endpoint: &str, args: &Value) -> Result<String, String> {
    let mut resolved = endpoint.to_string();
    if let Some(obj) = args.as_object() {
        for (key, val) in obj {
            let ph = format!("{{{key}}}");
            if let Some(s) = val.as_str() {
                resolved = resolved.replace(&ph, s);
            } else if let Some(n) = val.as_u64() {
                resolved = resolved.replace(&ph, &n.to_string());
            } else if let Some(n) = val.as_i64() {
                resolved = resolved.replace(&ph, &n.to_string());
            }
        }
    }
    // Split path from query string
    let (path, query) = match resolved.split_once('?') {
        Some((p, q)) => (p.to_string(), Some(q.to_string())),
        None => (resolved, None),
    };
    // Unresolved path params are an error
    if let Some(start) = path.find('{') {
        if let Some(end) = path[start..].find('}') {
            let param = &path[start + 1..start + end];
            return Err(format!("missing required path param: {param}"));
        }
    }
    // Strip unresolved query params
    match query {
        Some(q) => {
            let clean: Vec<&str> = q.split('&').filter(|p| !p.contains('{')).collect();
            if clean.is_empty() { Ok(path) } else { Ok(format!("{path}?{}", clean.join("&"))) }
        }
        None => Ok(path),
    }
}

fn daemon_url() -> String {
    std::env::var("DAEMON_URL").unwrap_or_else(|_| "http://localhost:8420".to_string())
}

fn make_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

fn auth_header() -> Option<String> {
    std::env::var("CONVERGIO_AUTH_TOKEN").ok()
}

fn http_get(url: &str) -> Option<String> {
    let mut req = make_client().get(url);
    if let Some(token) = auth_header() {
        req = req.header("Authorization", format!("Bearer {token}"));
    }
    match req.send() {
        Ok(resp) => {
            // Return error JSON for non-2xx so callers aren't silently fed error bodies (F-14)
            if resp.status().is_success() {
                Some(resp.text().unwrap_or_default())
            } else {
                let status = resp.status().as_u16();
                let body = resp.text().unwrap_or_default();
                warn!("kernel.tools GET {url}: HTTP {status}");
                Some(format!("{{\"error\":\"HTTP {status}\",\"body\":{body:?}}}"))
            }
        }
        Err(e) => {
            warn!("kernel.tools GET {url}: {e}");
            Some(format!("{{\"error\":\"{e}\"}}"))
        }
    }
}

fn http_post(url: &str, body: &Value) -> Option<String> {
    let mut req = make_client().post(url).json(body);
    if let Some(token) = auth_header() {
        req = req.header("Authorization", format!("Bearer {token}"));
    }
    match req.send() {
        Ok(resp) => {
            // Return error JSON for non-2xx so callers aren't silently fed error bodies (F-14)
            if resp.status().is_success() {
                Some(resp.text().unwrap_or_default())
            } else {
                let status = resp.status().as_u16();
                let body = resp.text().unwrap_or_default();
                warn!("kernel.tools POST {url}: HTTP {status}");
                Some(format!("{{\"error\":\"HTTP {status}\",\"body\":{body:?}}}"))
            }
        }
        Err(e) => {
            warn!("kernel.tools POST {url}: {e}");
            Some(format!("{{\"error\":\"{e}\"}}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_has_43_tools() {
        let cat = ToolCatalog::all();
        assert!(cat.tools.len() >= 43, "got {} tools", cat.tools.len());
    }

    #[test]
    fn test_catalog_find_existing() {
        assert!(ToolCatalog::all().find("get_plans").is_some());
    }

    #[test]
    fn test_catalog_find_missing() {
        assert!(ToolCatalog::all().find("nonexistent").is_none());
    }

    #[test]
    fn test_descriptions_block_nonempty() {
        assert!(ToolCatalog::all().descriptions_block().len() > 100);
    }

    #[test]
    fn test_resolve_endpoint_substitution() {
        let args = serde_json::json!({"plan_id": 42});
        let result = resolve_endpoint("/api/plan/{plan_id}", &args).unwrap();
        assert_eq!(result, "/api/plan/42");
    }

    #[test]
    fn test_resolve_endpoint_missing_optional_query() {
        let args = serde_json::json!({"to_agent": "kernel"});
        let result = resolve_endpoint(
            "/api/ipc/messages?to_agent={to_agent}&channel={channel}&limit={limit}",
            &args,
        ).unwrap();
        assert_eq!(result, "/api/ipc/messages?to_agent=kernel");
        assert!(!result.contains('{'));
    }

    #[test]
    fn test_resolve_endpoint_missing_path_param_errors() {
        let args = serde_json::json!({});
        let result = resolve_endpoint("/api/plan/{plan_id}/tasks", &args);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_only_excludes_write() {
        let cat = ToolCatalog::all();
        let read = cat.read_only_names();
        assert!(!read.contains(&"update_task"));
        assert!(read.contains(&"get_plans"));
    }
}
