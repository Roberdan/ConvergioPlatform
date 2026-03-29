// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Handler for cvg_invoke_agent MCP tool: reads agent definition, builds prompt, runs claude CLI.

use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;

use crate::mcp_server::security::McpError;

// ── Agent name validation ────────────────────────────────────────────────────

/// Extracts agent_name from tool arguments.
pub fn parse_agent_name(args: &Value) -> Result<&str, McpError> {
    args.get("agent_name")
        .and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("agent_name is required"))
}

/// Validates agent name: alphanumeric + hyphens only, no path traversal.
pub fn validate_agent_name(name: &str) -> Result<(), McpError> {
    if name.is_empty() {
        return Err(McpError::InvalidParams("agent_name must not be empty"));
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(McpError::InvalidParams("agent_name contains invalid characters"));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return Err(McpError::InvalidParams("agent_name must be alphanumeric, hyphens, underscores"));
    }
    Ok(())
}

// ── Frontmatter parsing ─────────────────────────────────────────────────────

/// Parses YAML frontmatter from agent definition, returns (model, body).
/// Defaults model to "claude-sonnet-4-6" if not specified.
pub fn parse_agent_frontmatter(content: &str) -> Result<(String, String), McpError> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Ok(("claude-sonnet-4-6".to_string(), content.to_string()));
    }
    let after_first = &trimmed[3..];
    let end = after_first
        .find("\n---")
        .ok_or(McpError::InvalidParams("malformed frontmatter: no closing ---"))?;
    let frontmatter = &after_first[..end];
    let body = after_first[end + 4..].trim_start().to_string();
    let model = frontmatter
        .lines()
        .find_map(|line| {
            let line = line.trim();
            if line.starts_with("model:") {
                Some(line["model:".len()..].trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "claude-sonnet-4-6".to_string());
    Ok((model, body))
}

// ── Agent file resolution ────────────────────────────────────────────────────

/// Search paths for agent definitions, in priority order.
fn agent_search_paths(name: &str) -> Vec<PathBuf> {
    vec![
        PathBuf::from(format!("claude-config/agents/{name}.md")),
        PathBuf::from(format!(".github/agents/{name}.agent.md")),
        PathBuf::from(format!(".claude/agents/{name}.md")),
    ]
}

/// Reads agent definition from the first matching path.
fn read_agent_definition(name: &str) -> Result<String, McpError> {
    for path in agent_search_paths(name) {
        if path.exists() {
            return std::fs::read_to_string(&path)
                .map_err(|e| McpError::DaemonError(format!("failed to read {}: {e}", path.display())));
        }
    }
    Err(McpError::InvalidParams("agent definition not found in any search path"))
}

// ── Build prompt ─────────────────────────────────────────────────────────────

fn build_prompt(agent_body: &str, task: &str, context: Option<&str>) -> String {
    let mut prompt = format!("# Agent Instructions\n\n{agent_body}\n\n# Task\n\n{task}");
    if let Some(ctx) = context {
        prompt.push_str(&format!("\n\n# Context\n\n{ctx}"));
    }
    prompt
}

// ── Execute claude CLI ───────────────────────────────────────────────────────

/// Runs `claude -p` with the given prompt and model. Returns stdout as JSON.
fn run_claude_cli(prompt: &str, model: &str) -> Result<Value, McpError> {
    let output = Command::new("claude")
        .args(["-p", prompt, "--model", model, "--output-format", "text"])
        .env("CLAUDE_NO_TELEMETRY", "1")
        .output()
        .map_err(|e| McpError::DaemonError(format!("failed to spawn claude CLI: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(McpError::DaemonError(format!("claude CLI exited {}: {stderr}", output.status)));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(json!({"agent_output": stdout, "model": model}))
}

// ── Public handler ───────────────────────────────────────────────────────────

/// MCP handler for cvg_invoke_agent. Reads agent def, builds prompt, runs claude.
pub fn invoke_agent(args: &Value) -> Result<Value, McpError> {
    let name = parse_agent_name(args)?;
    validate_agent_name(name)?;
    let task = args
        .get("task")
        .and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("task is required"))?;
    let context = args.get("context").and_then(|v| v.as_str());
    let content = read_agent_definition(name)?;
    let (model, body) = parse_agent_frontmatter(&content)?;
    let prompt = build_prompt(&body, task, context);
    run_claude_cli(&prompt, &model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validate_name_accepts_valid() {
        assert!(validate_agent_name("dario-debugger").is_ok());
        assert!(validate_agent_name("plan-reviewer").is_ok());
        assert!(validate_agent_name("agent_v2").is_ok());
    }

    #[test]
    fn validate_name_rejects_path_traversal() {
        assert!(validate_agent_name("../etc/passwd").is_err());
        assert!(validate_agent_name("foo/bar").is_err());
        assert!(validate_agent_name("").is_err());
    }

    #[test]
    fn parse_name_from_args() {
        let args = json!({"agent_name": "dario-debugger", "task": "fix bug"});
        assert_eq!(parse_agent_name(&args).unwrap(), "dario-debugger");
        assert!(parse_agent_name(&json!({"task": "fix bug"})).is_err());
    }

    #[test]
    fn frontmatter_extracts_model() {
        let content = "---\nname: test\nmodel: claude-sonnet-4-6\n---\n\n# Agent\nBody.";
        let (model, body) = parse_agent_frontmatter(content).unwrap();
        assert_eq!(model, "claude-sonnet-4-6");
        assert!(body.contains("# Agent"));
    }

    #[test]
    fn frontmatter_defaults_model() {
        let content = "---\nname: no-model\n---\n\nBody here.";
        let (model, _) = parse_agent_frontmatter(content).unwrap();
        assert_eq!(model, "claude-sonnet-4-6");
    }

    #[test]
    fn no_frontmatter_returns_full_body() {
        let content = "# Just markdown\nNo frontmatter.";
        let (model, body) = parse_agent_frontmatter(content).unwrap();
        assert_eq!(model, "claude-sonnet-4-6");
        assert_eq!(body, content);
    }

    #[test]
    fn build_prompt_with_context() {
        let p = build_prompt("Do X", "Fix Y", Some("Extra info"));
        assert!(p.contains("# Agent Instructions"));
        assert!(p.contains("Do X"));
        assert!(p.contains("Fix Y"));
        assert!(p.contains("Extra info"));
    }

    #[test]
    fn build_prompt_without_context() {
        let p = build_prompt("Do X", "Fix Y", None);
        assert!(!p.contains("# Context"));
    }
}
