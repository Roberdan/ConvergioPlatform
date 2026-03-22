// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Plan subcommands for cvg CLI — daemon HTTP API. Handlers in cli_plan_handlers.rs.
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
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Show plan JSON
    Show {
        /// Plan ID
        plan_id: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Check plan staleness (drift check)
    Drift {
        /// Plan ID
        plan_id: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Validate a wave in a plan (Thor)
    Validate {
        /// Plan ID
        plan_id: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Create a new plan
    Create {
        /// Project identifier
        project_id: String,
        /// Plan name
        name: String,
        /// Source spec file path
        #[arg(long)]
        source_file: Option<String>,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Import a spec YAML into a plan
    Import {
        /// Plan ID to import into
        plan_id: i64,
        /// Path to spec YAML file
        spec_file: String,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Start plan execution
    Start {
        /// Plan ID
        plan_id: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Mark plan as complete
    Complete {
        /// Plan ID
        plan_id: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Cancel a plan with reason
    Cancel {
        /// Plan ID
        plan_id: i64,
        /// Cancellation reason
        reason: String,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Approve a plan for execution
    Approve {
        /// Plan ID
        plan_id: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

pub async fn handle(cmd: PlanCommands) {
    crate::cli_plan_handlers::dispatch(cmd).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_commands_list_variant_exists() {
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

    #[test]
    fn plan_commands_create_variant_exists() {
        let with_src = PlanCommands::Create {
            project_id: "convergio".to_string(),
            name: "Migration Plan".to_string(),
            source_file: Some("/tmp/spec.yaml".to_string()),
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(with_src, PlanCommands::Create { .. }));
        // Verify None source_file is accepted
        let without_src = PlanCommands::Create {
            project_id: "convergio".to_string(),
            name: "Quick Plan".to_string(),
            source_file: None,
            human: true,
            api_url: "http://localhost:8420".to_string(),
        };
        if let PlanCommands::Create { source_file, .. } = &without_src {
            assert!(source_file.is_none());
        }
    }

    #[test]
    fn plan_commands_import_variant_exists() {
        let cmd = PlanCommands::Import {
            plan_id: 100,
            spec_file: "/tmp/spec.yaml".to_string(),
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, PlanCommands::Import { plan_id: 100, .. }));
    }

    #[test]
    fn plan_commands_start_variant_exists() {
        let cmd = PlanCommands::Start {
            plan_id: 200,
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, PlanCommands::Start { plan_id: 200, .. }));
    }

    #[test]
    fn plan_commands_complete_variant_exists() {
        let cmd = PlanCommands::Complete {
            plan_id: 300,
            human: true,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, PlanCommands::Complete { plan_id: 300, .. }));
    }

    #[test]
    fn plan_commands_cancel_variant_exists() {
        let cmd = PlanCommands::Cancel {
            plan_id: 400,
            reason: "Requirements changed".to_string(),
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, PlanCommands::Cancel { plan_id: 400, .. }));
    }

    #[test]
    fn plan_commands_approve_variant_exists() {
        let cmd = PlanCommands::Approve {
            plan_id: 500,
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, PlanCommands::Approve { plan_id: 500, .. }));
    }
}
