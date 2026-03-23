// cvg delegation — manage plan delegation to mesh workers.
// Why: Plan 706 — no orchestration for remote execution. Zero traceability.

use crate::cli_error::CliError;
use crate::cli_http;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum DelegationCommands {
    /// Show delegation status for a plan
    Status { plan_id: i64 },
    /// List all active plans (potential delegations)
    List,
}

fn api_err(code: i32) -> CliError {
    CliError::ApiCallFailed(format!("daemon returned error (code {code})"))
}

pub async fn handle(cmd: DelegationCommands, api_url: &str) -> Result<(), CliError> {
    match cmd {
        DelegationCommands::Status { plan_id } => {
            let url = format!("{api_url}/api/plan-db/json/{plan_id}");
            let body = cli_http::get_and_return(&url).await.map_err(api_err)?;
            let plan = &body["plan"];
            let host = plan["execution_host"].as_str().unwrap_or("none");
            let status = plan["status"].as_str().unwrap_or("?");
            let done = plan["tasks_done"].as_i64().unwrap_or(0);
            let total = plan["tasks_total"].as_i64().unwrap_or(0);
            println!("Plan {plan_id}: status={status}, host={host}, progress={done}/{total}");

            // Show workers for this plan
            let workers_url = format!("{api_url}/api/workers");
            if let Ok(wb) = cli_http::get_and_return(&workers_url).await {
                if let Some(list) = wb["workers"].as_array() {
                    let active: Vec<_> = list.iter().filter(|w| w["plan_id"].as_i64() == Some(plan_id)).collect();
                    if active.is_empty() {
                        println!("No active workers.");
                    } else {
                        for w in &active {
                            println!("  worker: {} on {}", w["agent_id"].as_str().unwrap_or("?"), w["host"].as_str().unwrap_or("?"));
                        }
                    }
                }
            }
            Ok(())
        }
        DelegationCommands::List => {
            let url = format!("{api_url}/api/plan-db/list");
            let body = cli_http::get_and_return(&url).await.map_err(api_err)?;
            if let Some(plans) = body["plans"].as_array() {
                let doing: Vec<_> = plans.iter().filter(|p| p["status"].as_str() == Some("doing")).collect();
                if doing.is_empty() {
                    println!("No active plans.");
                } else {
                    println!("{:<6} {:<45} {:<12} {}", "ID", "NAME", "HOST", "PROGRESS");
                    println!("{}", "-".repeat(80));
                    for p in &doing {
                        let id = p["id"].as_i64().unwrap_or(0);
                        let name = p["name"].as_str().unwrap_or("?");
                        let host = p["execution_host"].as_str().unwrap_or("-");
                        let done = p["tasks_done"].as_i64().unwrap_or(0);
                        let total = p["tasks_total"].as_i64().unwrap_or(0);
                        let display = if name.len() > 44 { &name[..44] } else { name };
                        println!("{:<6} {:<45} {:<12} {}/{}", id, display, host, done, total);
                    }
                }
            }
            Ok(())
        }
    }
}
