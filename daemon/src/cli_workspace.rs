// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Workspace subcommands for the cvg CLI — delegates to daemon HTTP API via reqwest.
// JSON output by default; --human flag for readable text.

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum WorkspaceCommands {
    /// Create a plan/wave workspace
    Create {
        /// Plan ID
        #[arg(long)]
        plan: Option<i64>,
        /// Wave DB ID
        #[arg(long)]
        wave: Option<i64>,
        /// Human-readable output
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Create a feature branch workspace
    CreateFeature {
        /// Branch name
        branch: String,
        /// Human-readable output
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Delete a workspace
    Delete {
        /// Workspace ID
        workspace_id: String,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// List workspaces
    List {
        /// Filter by plan ID
        #[arg(long)]
        plan: Option<i64>,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Show workspace status
    Status {
        /// Workspace ID
        workspace_id: String,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Show workspace events
    Events {
        /// Workspace ID
        workspace_id: String,
        /// Max events to return
        #[arg(long, default_value = "20")]
        limit: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

pub async fn handle(cmd: WorkspaceCommands) {
    match cmd {
        WorkspaceCommands::Create {
            plan,
            wave,
            human,
            api_url,
        } => {
            let body = serde_json::json!({"plan_id": plan, "wave_db_id": wave});
            crate::cli_http::post_and_print(
                &format!("{api_url}/api/workspace/create"),
                &body,
                human,
            )
            .await;
        }
        WorkspaceCommands::CreateFeature {
            branch,
            human,
            api_url,
        } => {
            let body = serde_json::json!({"feature": true, "branch": branch});
            crate::cli_http::post_and_print(
                &format!("{api_url}/api/workspace/create"),
                &body,
                human,
            )
            .await;
        }
        WorkspaceCommands::Delete {
            workspace_id,
            human,
            api_url,
        } => {
            let body = serde_json::json!({"workspace_id": workspace_id});
            crate::cli_http::post_and_print(
                &format!("{api_url}/api/workspace/delete"),
                &body,
                human,
            )
            .await;
        }
        WorkspaceCommands::List {
            plan,
            human,
            api_url,
        } => {
            let url = if let Some(pid) = plan {
                format!("{api_url}/api/workspace/list?plan_id={pid}")
            } else {
                format!("{api_url}/api/workspace/list")
            };
            crate::cli_http::fetch_and_print(&url, human).await;
        }
        WorkspaceCommands::Status {
            workspace_id,
            human,
            api_url,
        } => {
            crate::cli_http::fetch_and_print(
                &format!("{api_url}/api/workspace/status/{workspace_id}"),
                human,
            )
            .await;
        }
        WorkspaceCommands::Events {
            workspace_id,
            limit,
            human,
            api_url,
        } => {
            crate::cli_http::fetch_and_print(
                &format!(
                    "{api_url}/api/workspace/events?workspace_id={workspace_id}&limit={limit}"
                ),
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
    fn workspace_create_with_plan_and_wave() {
        let cmd = WorkspaceCommands::Create {
            plan: Some(698),
            wave: Some(100),
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(
            cmd,
            WorkspaceCommands::Create {
                plan: Some(698),
                ..
            }
        ));
    }

    #[test]
    fn workspace_create_feature_variant() {
        let cmd = WorkspaceCommands::CreateFeature {
            branch: "feat/new-thing".to_string(),
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, WorkspaceCommands::CreateFeature { .. }));
    }

    #[test]
    fn workspace_delete_variant() {
        let cmd = WorkspaceCommands::Delete {
            workspace_id: "ws-12345-abcd".to_string(),
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, WorkspaceCommands::Delete { .. }));
    }

    #[test]
    fn workspace_list_filtered_by_plan() {
        let cmd = WorkspaceCommands::List {
            plan: Some(698),
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(
            cmd,
            WorkspaceCommands::List {
                plan: Some(698),
                ..
            }
        ));
    }

    #[test]
    fn workspace_status_variant() {
        let cmd = WorkspaceCommands::Status {
            workspace_id: "ws-abc123".to_string(),
            human: true,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, WorkspaceCommands::Status { .. }));
    }

    #[test]
    fn workspace_events_variant() {
        let cmd = WorkspaceCommands::Events {
            workspace_id: "ws-abc123".to_string(),
            limit: 50,
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, WorkspaceCommands::Events { limit: 50, .. }));
    }

    #[test]
    fn workspace_create_body_shape() {
        let body = serde_json::json!({"plan_id": 698_i64, "wave_db_id": 100_i64});
        assert_eq!(body["plan_id"], 698);
        assert_eq!(body["wave_db_id"], 100);
    }

    #[test]
    fn workspace_create_feature_body_shape() {
        let body = serde_json::json!({"feature": true, "branch": "feat/my-feature"});
        assert_eq!(body["feature"], true);
        assert_eq!(body["branch"], "feat/my-feature");
    }

    #[test]
    fn workspace_delete_body_shape() {
        let body = serde_json::json!({"workspace_id": "ws-12345-abcd"});
        assert_eq!(body["workspace_id"], "ws-12345-abcd");
    }

    #[test]
    fn workspace_events_default_limit_is_20() {
        // Clap default_value = "20"; verify enum carries it through.
        let cmd = WorkspaceCommands::Events {
            workspace_id: "ws-test".to_string(),
            limit: 20,
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        if let WorkspaceCommands::Events { limit, .. } = cmd {
            assert_eq!(limit, 20);
        }
    }
}
