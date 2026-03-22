// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Handler dispatch for plan subcommands — split from cli_plan.rs for 250-line limit.

use crate::cli_plan::PlanCommands;

pub async fn dispatch(cmd: PlanCommands) {
    match cmd {
        // --- GET-based subcommands ---
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
        // --- POST-based subcommands ---
        PlanCommands::Create { project_id, name, source_file, human, api_url } => {
            let body = serde_json::json!({
                "project_id": project_id,
                "name": name,
                "source_file": source_file,
            });
            crate::cli_http::post_and_print(
                &format!("{api_url}/api/plan-db/create"),
                &body,
                human,
            )
            .await;
        }
        PlanCommands::Import { plan_id, spec_file, human, api_url } => {
            let content = match std::fs::read_to_string(&spec_file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error reading spec file '{spec_file}': {e}");
                    std::process::exit(2);
                }
            };
            let body = serde_json::json!({
                "plan_id": plan_id,
                "spec": content,
            });
            crate::cli_http::post_and_print(
                &format!("{api_url}/api/plan-db/import"),
                &body,
                human,
            )
            .await;
        }
        PlanCommands::Start { plan_id, human, api_url } => {
            let body = serde_json::json!({});
            crate::cli_http::post_and_print(
                &format!("{api_url}/api/plan-db/start/{plan_id}"),
                &body,
                human,
            )
            .await;
        }
        PlanCommands::Complete { plan_id, human, api_url } => {
            let body = serde_json::json!({});
            crate::cli_http::post_and_print(
                &format!("{api_url}/api/plan-db/complete/{plan_id}"),
                &body,
                human,
            )
            .await;
        }
        PlanCommands::Cancel { plan_id, reason, human, api_url } => {
            let body = serde_json::json!({ "reason": reason });
            crate::cli_http::post_and_print(
                &format!("{api_url}/api/plan-db/cancel/{plan_id}"),
                &body,
                human,
            )
            .await;
        }
        PlanCommands::Approve { plan_id, human, api_url } => {
            let body = serde_json::json!({});
            crate::cli_http::post_and_print(
                &format!("{api_url}/api/plan-db/approve/{plan_id}"),
                &body,
                human,
            )
            .await;
        }
        PlanCommands::Readiness { plan_id, human, api_url } => {
            crate::cli_http::fetch_and_print(
                &format!("{api_url}/api/plan-db/readiness/{plan_id}"),
                human,
            )
            .await;
        }
    }
}
