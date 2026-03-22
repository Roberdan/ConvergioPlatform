// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Checkpoint, Lock, and Review subcommands for the cvg CLI.
// Delegates to daemon HTTP API via reqwest.
// JSON output by default; --human flag for readable text.

use clap::Subcommand;

// --- Checkpoint subcommands ---

#[derive(Debug, Subcommand)]
pub enum CheckpointCommands {
    /// Save current plan state to checkpoint file
    Save {
        /// Plan ID
        plan_id: i64,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Restore plan state from checkpoint file
    Restore {
        /// Plan ID
        plan_id: i64,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

pub async fn handle_checkpoint(cmd: CheckpointCommands) {
    match cmd {
        CheckpointCommands::Save { plan_id, human, api_url } => {
            let body = serde_json::json!({ "plan_id": plan_id });
            post_and_print(
                &format!("{api_url}/api/plan-db/checkpoint/save"),
                &body,
                human,
            )
            .await;
        }
        CheckpointCommands::Restore { plan_id, human, api_url } => {
            fetch_and_print(
                &format!("{api_url}/api/plan-db/checkpoint/restore?plan_id={plan_id}"),
                human,
            )
            .await;
        }
    }
}

// --- Lock subcommands ---

#[derive(Debug, Subcommand)]
pub enum LockCommands {
    /// Acquire a file lock for a task
    Acquire {
        /// File path to lock
        file_path: String,
        /// Task DB ID that owns this lock
        task_id: i64,
        /// Agent identifier
        #[arg(long, default_value = "task-executor")]
        agent: String,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Release a file lock
    Release {
        /// File path to unlock
        file_path: String,
        /// Task DB ID releasing this lock
        task_id: i64,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// List all active file locks
    List {
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

pub async fn handle_lock(cmd: LockCommands) {
    match cmd {
        LockCommands::Acquire { file_path, task_id, agent, human, api_url } => {
            let body = serde_json::json!({
                "file_path": file_path,
                "task_id": task_id,
                "agent": agent,
            });
            post_and_print(&format!("{api_url}/api/ipc/locks/acquire"), &body, human).await;
        }
        LockCommands::Release { file_path, task_id, human, api_url } => {
            let body = serde_json::json!({
                "file_path": file_path,
                "task_id": task_id,
            });
            post_and_print(&format!("{api_url}/api/ipc/locks/release"), &body, human).await;
        }
        LockCommands::List { human, api_url } => {
            fetch_and_print(&format!("{api_url}/api/ipc/locks"), human).await;
        }
    }
}

// --- Review subcommands ---

#[derive(Debug, Subcommand)]
pub enum ReviewCommands {
    /// Register a plan review record
    Register {
        /// Plan ID
        plan_id: i64,
        /// Reviewer agent name (e.g. plan-reviewer, plan-business-advisor)
        reviewer_agent: String,
        /// Verdict (approved, rejected, proceed)
        verdict: String,
        /// Optional suggestions text
        #[arg(long)]
        suggestions: Option<String>,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Check review counts for a plan
    Check {
        /// Plan ID
        plan_id: i64,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Reset (delete) all reviews for a plan
    Reset {
        /// Plan ID
        plan_id: i64,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

pub async fn handle_review(cmd: ReviewCommands) {
    match cmd {
        ReviewCommands::Register { plan_id, reviewer_agent, verdict, suggestions, human, api_url } => {
            let body = serde_json::json!({
                "plan_id": plan_id,
                "reviewer_agent": reviewer_agent,
                "verdict": verdict,
                "suggestions": suggestions,
            });
            post_and_print(
                &format!("{api_url}/api/plan-db/review/register"),
                &body,
                human,
            )
            .await;
        }
        ReviewCommands::Check { plan_id, human, api_url } => {
            fetch_and_print(
                &format!("{api_url}/api/plan-db/review/check?plan_id={plan_id}"),
                human,
            )
            .await;
        }
        ReviewCommands::Reset { plan_id, human, api_url } => {
            let body = serde_json::json!({ "plan_id": plan_id });
            post_and_print(
                &format!("{api_url}/api/plan-db/review/reset"),
                &body,
                human,
            )
            .await;
        }
    }
}

// --- Shared HTTP helpers ---

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
        println!(
            "{}",
            serde_json::to_string_pretty(val).unwrap_or_else(|_| val.to_string())
        );
    } else {
        println!("{val}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Checkpoint ---

    #[test]
    fn checkpoint_save_variant_exists() {
        let cmd = CheckpointCommands::Save {
            plan_id: 685,
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, CheckpointCommands::Save { plan_id: 685, .. }));
    }

    #[test]
    fn checkpoint_restore_variant_exists() {
        let cmd = CheckpointCommands::Restore {
            plan_id: 42,
            human: true,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, CheckpointCommands::Restore { plan_id: 42, .. }));
    }

    // --- Lock ---

    #[test]
    fn lock_acquire_variant_exists() {
        let cmd = LockCommands::Acquire {
            file_path: "daemon/src/main.rs".to_string(),
            task_id: 8796,
            agent: "task-executor".to_string(),
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, LockCommands::Acquire { task_id: 8796, .. }));
    }

    #[test]
    fn lock_release_variant_exists() {
        let cmd = LockCommands::Release {
            file_path: "daemon/src/main.rs".to_string(),
            task_id: 8796,
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, LockCommands::Release { task_id: 8796, .. }));
    }

    #[test]
    fn lock_list_variant_exists() {
        let cmd = LockCommands::List {
            human: true,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, LockCommands::List { human: true, .. }));
    }

    // --- Review ---

    #[test]
    fn review_register_variant_exists() {
        let cmd = ReviewCommands::Register {
            plan_id: 685,
            reviewer_agent: "plan-reviewer".to_string(),
            verdict: "approved".to_string(),
            suggestions: None,
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, ReviewCommands::Register { plan_id: 685, .. }));
    }

    #[test]
    fn review_check_variant_exists() {
        let cmd = ReviewCommands::Check {
            plan_id: 100,
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, ReviewCommands::Check { plan_id: 100, .. }));
    }

    #[test]
    fn review_reset_variant_exists() {
        let cmd = ReviewCommands::Reset {
            plan_id: 1,
            human: true,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, ReviewCommands::Reset { plan_id: 1, .. }));
    }

    #[test]
    fn checkpoint_save_body_shape() {
        let body = serde_json::json!({ "plan_id": 685_i64 });
        assert_eq!(body["plan_id"], 685);
    }

    #[test]
    fn lock_acquire_body_shape() {
        let body = serde_json::json!({
            "file_path": "daemon/src/main.rs",
            "task_id": 8796_i64,
            "agent": "task-executor",
        });
        assert_eq!(body["task_id"], 8796);
        assert_eq!(body["agent"], "task-executor");
    }

    #[test]
    fn review_register_body_shape() {
        let body = serde_json::json!({
            "plan_id": 685_i64,
            "reviewer_agent": "plan-reviewer",
            "verdict": "approved",
            "suggestions": serde_json::Value::Null,
        });
        assert_eq!(body["verdict"], "approved");
    }
}
