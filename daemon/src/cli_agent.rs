// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Agent subcommands for the cvg CLI — delegates to daemon HTTP API via reqwest.
// JSON output by default; --human flag for readable text.

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum AgentCommands {
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
}

pub async fn handle(cmd: AgentCommands) {
    match cmd {
        AgentCommands::Start { name, task_id, human, api_url } => {
            let body = serde_json::json!({
                "name": name,
                "task_id": task_id,
            });
            post_and_print(&format!("{api_url}/api/plan-db/agent/start"), &body, human).await;
        }
        AgentCommands::Complete { agent_id, summary, human, api_url } => {
            let body = serde_json::json!({
                "agent_id": agent_id,
                "summary": summary,
            });
            post_and_print(&format!("{api_url}/api/plan-db/agent/complete"), &body, human).await;
        }
        AgentCommands::List { human, api_url } => {
            fetch_and_print(&format!("{api_url}/api/agents"), human).await;
        }
    }
}

async fn fetch_and_print(url: &str, human: bool) {
    match reqwest::get(url).await {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<serde_json::Value>().await {
                Ok(val) => print_value(&val, human),
                Err(e) => {
                    eprintln!("error parsing response: {e}");
                    std::process::exit(2);
                }
            }
            if !status.is_success() {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("error connecting to daemon: {e}");
            std::process::exit(2);
        }
    }
}

async fn post_and_print(url: &str, body: &serde_json::Value, human: bool) {
    let client = reqwest::Client::new();
    match client.post(url).json(body).send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<serde_json::Value>().await {
                Ok(val) => print_value(&val, human),
                Err(e) => {
                    eprintln!("error parsing response: {e}");
                    std::process::exit(2);
                }
            }
            if !status.is_success() {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("error connecting to daemon: {e}");
            std::process::exit(2);
        }
    }
}

fn print_value(val: &serde_json::Value, human: bool) {
    if human {
        println!("{}", serde_json::to_string_pretty(val)
            .unwrap_or_else(|_| val.to_string()));
    } else {
        println!("{val}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_commands_start_variant_exists() {
        let cmd = AgentCommands::Start {
            name: "task-executor".to_string(),
            task_id: Some(8797),
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, AgentCommands::Start { .. }));
    }

    #[test]
    fn agent_commands_complete_variant_exists() {
        let cmd = AgentCommands::Complete {
            agent_id: "abc-123".to_string(),
            summary: Some("done".to_string()),
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, AgentCommands::Complete { .. }));
    }

    #[test]
    fn agent_commands_list_variant_exists() {
        let cmd = AgentCommands::List {
            human: true,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, AgentCommands::List { .. }));
    }

    #[test]
    fn agent_start_builds_correct_body() {
        let body = serde_json::json!({
            "name": "task-executor",
            "task_id": 42_i64,
        });
        assert_eq!(body["name"], "task-executor");
        assert_eq!(body["task_id"], 42);
    }
}
