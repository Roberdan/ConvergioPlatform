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
            crate::cli_http::fetch_and_print(
                &format!("{api_url}/api/plan-db/validate-task/{task_id}/{plan_id}"),
                human,
            )
            .await;
        }
        TaskCommands::KbSearch { query, limit, human, api_url } => {
            crate::cli_http::fetch_and_print(
                &format!("{api_url}/api/plan-db/kb-search?q={query}&limit={limit}"),
                human,
            )
            .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_commands_update_variant_exists() {
        let cmd = TaskCommands::Update {
            task_id: 100,
            status: "done".to_string(),
            summary: Some("finished".to_string()),
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, TaskCommands::Update { task_id: 100, .. }));
    }

    #[test]
    fn task_commands_validate_variant_exists() {
        let cmd = TaskCommands::Validate {
            task_id: 1,
            plan_id: 685,
            human: true,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, TaskCommands::Validate { plan_id: 685, .. }));
    }

    #[test]
    fn task_commands_kb_search_variant_exists() {
        let cmd = TaskCommands::KbSearch {
            query: "test".to_string(),
            limit: 5,
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, TaskCommands::KbSearch { .. }));
    }

    #[test]
    fn print_value_json_compact() {
        let val = serde_json::json!({"ok": true, "data": [1, 2]});
        // Compact: no newlines in outer structure
        let compact = val.to_string();
        assert!(!compact.is_empty());
    }

    #[test]
    fn print_value_json_pretty() {
        let val = serde_json::json!({"ok": true});
        let pretty = serde_json::to_string_pretty(&val).unwrap();
        assert!(pretty.contains('\n'));
    }
}
