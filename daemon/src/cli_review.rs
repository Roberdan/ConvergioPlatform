// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Review subcommand — register/check/reset plan reviews via daemon HTTP API.

use clap::Subcommand;

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
    /// Reset (delete) all reviews for a plan (omit plan_id to reset pre-plan state)
    Reset {
        /// Plan ID (optional — omit to reset without a plan, e.g. before cvg plan create)
        plan_id: Option<i64>,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

pub async fn handle(cmd: ReviewCommands) {
    match cmd {
        ReviewCommands::Register { plan_id, reviewer_agent, verdict, suggestions, human, api_url } => {
            let body = serde_json::json!({
                "plan_id": plan_id, "reviewer_agent": reviewer_agent,
                "verdict": verdict, "suggestions": suggestions,
            });
            crate::cli_http::post_and_print(
                &format!("{api_url}/api/plan-db/review/register"), &body, human,
            ).await;
        }
        ReviewCommands::Check { plan_id, human, api_url } => {
            crate::cli_http::fetch_and_print(
                &format!("{api_url}/api/plan-db/review/check?plan_id={plan_id}"), human,
            ).await;
        }
        ReviewCommands::Reset { plan_id, human, api_url } => {
            let body = serde_json::json!({ "plan_id": plan_id.unwrap_or(0) });
            crate::cli_http::post_and_print(
                &format!("{api_url}/api/plan-db/review/reset"), &body, human,
            ).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_register_variant_exists() {
        let cmd = ReviewCommands::Register {
            plan_id: 685, reviewer_agent: "plan-reviewer".to_string(),
            verdict: "approved".to_string(), suggestions: None,
            human: false, api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, ReviewCommands::Register { plan_id: 685, .. }));
    }

    #[test]
    fn review_check_variant_exists() {
        let cmd = ReviewCommands::Check {
            plan_id: 100, human: false, api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, ReviewCommands::Check { plan_id: 100, .. }));
    }

    #[test]
    fn review_reset_variant_exists() {
        let cmd = ReviewCommands::Reset {
            plan_id: Some(1), human: true, api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, ReviewCommands::Reset { plan_id: Some(1), .. }));
    }

    #[test]
    fn review_reset_without_plan_id() {
        let cmd = ReviewCommands::Reset {
            plan_id: None, human: false, api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, ReviewCommands::Reset { plan_id: None, .. }));
    }

    #[test]
    fn review_register_body_shape() {
        let body = serde_json::json!({
            "plan_id": 685_i64, "reviewer_agent": "plan-reviewer",
            "verdict": "approved", "suggestions": serde_json::Value::Null,
        });
        assert_eq!(body["verdict"], "approved");
    }
}
