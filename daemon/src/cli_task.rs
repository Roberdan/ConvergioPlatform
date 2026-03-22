// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Task subcommands for the cvg CLI — delegates to daemon HTTP API via reqwest.
// JSON output by default; --human flag for readable text.

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum TaskCommands {
    /// Update a task status
    Update {
        /// Task DB ID
        task_id: i64,
        /// New status (e.g. in_progress, done, blocked)
        status: String,
        /// Optional summary message
        #[arg(long)]
        summary: Option<String>,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Validate a task (Thor gate)
    Validate {
        /// Task DB ID
        task_id: i64,
        /// Plan ID
        plan_id: i64,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Search the knowledge base
    KbSearch {
        /// Search query
        query: String,
        /// Maximum results to return
        #[arg(long, default_value_t = 5)]
        limit: u32,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Approve the deliverable linked to a task
    Approve {
        /// Task DB ID
        task_id: i64,
        /// Approver name or comment
        #[arg(long)]
        comment: Option<String>,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

pub async fn handle(cmd: TaskCommands) {
    match cmd {
        TaskCommands::Update { task_id, status, summary, human, api_url } => {
            let body = serde_json::json!({
                "task_id": task_id,
                "status": status,
                "summary": summary,
            });
            crate::cli_http::post_and_print(&format!("{api_url}/api/plan-db/task/update"), &body, human).await;
        }
        TaskCommands::Validate { task_id, plan_id, human, api_url } => {
            let url = format!("{api_url}/api/plan-db/validate-task/{task_id}/{plan_id}");
            match reqwest::get(&url).await {
                Ok(resp) => match resp.json::<serde_json::Value>().await {
                    Ok(val) => {
                        if human {
                            print_mechanical_human(&val);
                        } else {
                            println!("{val}");
                        }
                        // Exit 1 if mechanical gates rejected
                        let rejected = val
                            .get("mechanical")
                            .and_then(|m| m.get("status"))
                            .and_then(serde_json::Value::as_str)
                            == Some("REJECTED");
                        if rejected {
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("error parsing response: {e}");
                        std::process::exit(2);
                    }
                },
                Err(e) => {
                    eprintln!("error connecting to daemon: {e}");
                    std::process::exit(2);
                }
            }
        }
        TaskCommands::KbSearch { query, limit, human, api_url } => {
            crate::cli_http::fetch_and_print(
                &format!("{api_url}/api/plan-db/kb-search?q={query}&limit={limit}"),
                human,
            )
            .await;
        }
        TaskCommands::Approve { task_id, comment, human, api_url } => {
            crate::cli_task_approve::handle(task_id, comment, human, &api_url).await;
        }
    }
}

/// Human-readable output for mechanical gate validation results.
fn print_mechanical_human(val: &serde_json::Value) {
    let mechanical = match val.get("mechanical") {
        Some(m) => m,
        None => {
            println!("{}", serde_json::to_string_pretty(val).unwrap_or_default());
            return;
        }
    };

    let status = mechanical
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("UNKNOWN");
    let note = mechanical
        .get("note")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");

    println!("Mechanical Validation: {status}");
    println!();

    if let Some(gates) = mechanical.get("gates").and_then(|g| g.as_array()) {
        for gate in gates {
            let name = gate.get("gate").and_then(|g| g.as_str()).unwrap_or("?");
            let passed = gate.get("passed").and_then(|p| p.as_bool()).unwrap_or(false);
            let icon = if passed { "PASS" } else { "FAIL" };
            println!("  [{icon}] {name}");
            if let Some(details) = gate.get("details").and_then(|d| d.as_array()) {
                for d in details {
                    if let Some(s) = d.as_str() {
                        println!("         {s}");
                    }
                }
            }
        }
    }

    if !note.is_empty() {
        println!();
        println!("{note}");
    }
}

#[cfg(test)]
#[path = "cli_task_tests.rs"]
mod tests;
