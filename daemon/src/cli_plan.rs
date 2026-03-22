// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Plan subcommands for the cvg CLI — delegates to daemon HTTP API via reqwest.
// JSON output by default; --human flag for readable text.

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum PlanCommands {
    /// List active plans
    List {
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Show execution tree for a plan
    Tree {
        /// Plan ID
        plan_id: i64,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Show plan JSON
    Show {
        /// Plan ID
        plan_id: i64,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Check plan staleness (drift check)
    Drift {
        /// Plan ID
        plan_id: i64,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Validate a wave in a plan (Thor)
    Validate {
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

pub async fn handle(cmd: PlanCommands) {
    match cmd {
        PlanCommands::List { human, api_url } => {
            crate::cli_http::fetch_and_print(&format!("{api_url}/api/plan-db/list"), human).await;
        }
        PlanCommands::Tree { plan_id, human, api_url } => {
            crate::cli_http::fetch_and_print(
                &format!("{api_url}/api/plan-db/execution-tree/{plan_id}"),
                human,
            )
            .await;
        }
        PlanCommands::Show { plan_id, human, api_url } => {
            crate::cli_http::fetch_and_print(
                &format!("{api_url}/api/plan-db/json/{plan_id}"),
                human,
            )
            .await;
        }
        PlanCommands::Drift { plan_id, human, api_url } => {
            crate::cli_http::fetch_and_print(
                &format!("{api_url}/api/plan-db/drift-check/{plan_id}"),
                human,
            )
            .await;
        }
        PlanCommands::Validate { plan_id, human, api_url } => {
            crate::cli_http::fetch_and_print(
                &format!("{api_url}/api/plans/{plan_id}/validate"),
                human,
            )
            .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify PlanCommands variants exist and are parseable via clap derive.
    #[test]
    fn plan_commands_list_variant_exists() {
        // This test confirms the enum compiles with the expected variants.
        let cmd = PlanCommands::List {
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, PlanCommands::List { .. }));
    }

    #[test]
    fn plan_commands_tree_variant_exists() {
        let cmd = PlanCommands::Tree {
            plan_id: 42,
            human: true,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, PlanCommands::Tree { plan_id: 42, .. }));
    }

    #[test]
    fn plan_commands_show_variant_exists() {
        let cmd = PlanCommands::Show {
            plan_id: 1,
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, PlanCommands::Show { .. }));
    }

    #[test]
    fn plan_commands_drift_variant_exists() {
        let cmd = PlanCommands::Drift {
            plan_id: 5,
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, PlanCommands::Drift { .. }));
    }

    #[test]
    fn plan_commands_validate_variant_exists() {
        let cmd = PlanCommands::Validate {
            plan_id: 10,
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, PlanCommands::Validate { .. }));
    }
}
