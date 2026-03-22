// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Agent subcommands for the cvg CLI — delegates to daemon HTTP API via reqwest.
// JSON output by default; --human flag for readable text.

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum AgentCommands {
    /// Transpile an agent from the catalog to a provider-specific format
    Transpile {
        /// Agent name (looked up via /api/agents/catalog)
        name: String,
        /// Target provider: claude-code, copilot-cli, generic-llm
        #[arg(long, default_value = "claude-code")]
        provider: String,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Start a new agent session
    Start {
        /// Agent name or type
        name: String,
        /// Task ID this agent is working on
        #[arg(long)]
        task_id: Option<i64>,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Complete an active agent session
    Complete {
        /// Agent session ID
        agent_id: String,
        /// Completion summary
        #[arg(long)]
        summary: Option<String>,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// List active agents
    List {
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Sync agent catalog from .agent.md files in a directory
    Sync {
        /// Directory containing .agent.md files
        source_dir: String,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Enable an agent from catalog (write .agent.md to target dir)
    Enable {
        /// Agent name from catalog
        name: String,
        /// Directory to write the .agent.md file to
        #[arg(long)]
        target_dir: Option<String>,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Disable an agent (remove .agent.md from target dir)
    Disable {
        /// Agent name
        name: String,
        /// Directory containing the .agent.md file
        #[arg(long)]
        target_dir: Option<String>,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// List agents in the catalog
    Catalog {
        /// Filter by category
        #[arg(long)]
        category: Option<String>,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Triage a problem — find the best agent for a given task
    Triage {
        /// Description of the problem to solve
        description: String,
        /// Optional domain filter (e.g. "technical", "core")
        #[arg(long)]
        domain: Option<String>,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Create a new agent in the catalog
    Create {
        /// Agent name
        name: String,
        /// Agent category
        #[arg(long, default_value = "")]
        category: String,
        /// Agent description
        #[arg(long, default_value = "")]
        description: String,
        /// Model to use
        #[arg(long, default_value = "claude-sonnet-4-6")]
        model: String,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

pub async fn handle(cmd: AgentCommands) {
    match cmd {
        AgentCommands::Transpile { name, provider, api_url } => {
            handle_transpile(&name, &provider, &api_url).await;
        }
        AgentCommands::Start { name, task_id, human, api_url } => {
            let body = serde_json::json!({
                "name": name,
                "task_id": task_id,
            });
            crate::cli_http::post_and_print(&format!("{api_url}/api/plan-db/agent/start"), &body, human).await;
        }
        AgentCommands::Complete { agent_id, summary, human, api_url } => {
            let body = serde_json::json!({
                "agent_id": agent_id,
                "summary": summary,
            });
            crate::cli_http::post_and_print(&format!("{api_url}/api/plan-db/agent/complete"), &body, human).await;
        }
        AgentCommands::List { human, api_url } => {
            crate::cli_http::fetch_and_print(&format!("{api_url}/api/agents"), human).await;
        }
        AgentCommands::Sync { source_dir, human, api_url } => {
            let body = serde_json::json!({"source_dir": source_dir});
            crate::cli_http::post_and_print(&format!("{api_url}/api/agents/sync"), &body, human).await;
        }
        AgentCommands::Enable { name, target_dir, human, api_url } => {
            let dir = target_dir.unwrap_or_else(|| ".github/agents".to_string());
            let body = serde_json::json!({"name": name, "target_dir": dir});
            crate::cli_http::post_and_print(&format!("{api_url}/api/agents/enable"), &body, human).await;
        }
        AgentCommands::Disable { name, target_dir, human, api_url } => {
            let dir = target_dir.unwrap_or_else(|| ".github/agents".to_string());
            let body = serde_json::json!({"name": name, "target_dir": dir});
            crate::cli_http::post_and_print(&format!("{api_url}/api/agents/disable"), &body, human).await;
        }
        AgentCommands::Catalog { category, human, api_url } => {
            let url = if let Some(cat) = category {
                format!("{api_url}/api/agents/catalog?category={cat}")
            } else {
                format!("{api_url}/api/agents/catalog")
            };
            crate::cli_http::fetch_and_print(&url, human).await;
        }
        AgentCommands::Triage { description, domain, human, api_url } => {
            let body = serde_json::json!({
                "problem_description": description,
                "domain": domain,
            });
            crate::cli_http::post_and_print(&format!("{api_url}/api/agents/triage"), &body, human).await;
        }
        AgentCommands::Create { name, category, description, model, human, api_url } => {
            let body = serde_json::json!({
                "name": name, "category": category,
                "description": description, "model": model,
            });
            crate::cli_http::post_and_print(&format!("{api_url}/api/agents/create"), &body, human).await;
        }
    }
}

async fn handle_transpile(name: &str, provider: &str, api_url: &str) {
    let enc: String = name.chars().map(|c| {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c.to_string() }
        else { format!("%{:02X}", c as u32) }
    }).collect();
    let url = format!("{api_url}/api/agents/catalog?name={enc}");
    let resp = reqwest::get(&url).await.unwrap_or_else(|e| {
        eprintln!("error connecting to daemon: {e}"); std::process::exit(2);
    });
    let val: serde_json::Value = resp.json().await.unwrap_or_else(|e| {
        eprintln!("error parsing response: {e}"); std::process::exit(2);
    });
    let agent = if val.is_array() { val.as_array().and_then(|a| a.first()).cloned() }
        else { Some(val) };
    let agent = agent.unwrap_or_else(|| {
        eprintln!("agent '{name}' not found in catalog"); std::process::exit(1);
    });
    let desc = agent["description"].as_str().unwrap_or("");
    let model = agent["model"].as_str().unwrap_or("claude-sonnet-4-6");
    let tools = agent["tools"].as_str().unwrap_or("view,edit,bash");
    let output = match provider {
        "claude-code" => crate::transpiler::transpile_claude_code(name, desc, model, tools),
        "copilot-cli" => crate::transpiler::transpile_copilot_cli(name, desc, model, tools),
        "generic-llm" => crate::transpiler::transpile_generic_llm(name, desc, model),
        other => { eprintln!("unknown provider '{other}'; use: claude-code, copilot-cli, generic-llm"); std::process::exit(2); }
    };
    print!("{output}");
}

#[cfg(test)]
#[path = "cli_agent_tests.rs"]
mod tests;
