// cvg delegation — manage plan delegation to mesh workers.
// Why: Plan 706 — no orchestration for remote execution. Zero traceability.
// Why Plan 720: adds `delegate` subcommand so operators don't need the bash script.

use crate::cli_error::CliError;
use crate::cli_http;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum DelegationCommands {
    /// Show delegation status for a plan
    Status { plan_id: i64 },
    /// List all active plans (potential delegations)
    List,
    /// Delegate a prompt to a mesh peer (POST /api/mesh/delegate)
    Delegate {
        /// Target peer hostname (e.g. macProM1)
        #[arg(long)]
        peer: String,
        /// Prompt or task description to run on the peer
        #[arg(long)]
        prompt: String,
        /// Associate with an existing plan ID (optional)
        #[arg(long)]
        plan_id: Option<i64>,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
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
                    let active: Vec<_> = list
                        .iter()
                        .filter(|w| w["plan_id"].as_i64() == Some(plan_id))
                        .collect();
                    if active.is_empty() {
                        println!("No active workers.");
                    } else {
                        for w in &active {
                            println!(
                                "  worker: {} on {}",
                                w["agent_id"].as_str().unwrap_or("?"),
                                w["host"].as_str().unwrap_or("?")
                            );
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
                let doing: Vec<_> = plans
                    .iter()
                    .filter(|p| p["status"].as_str() == Some("doing"))
                    .collect();
                if doing.is_empty() {
                    println!("No active plans.");
                } else {
                    println!("{:<6} {:<45} {:<12} PROGRESS", "ID", "NAME", "HOST");
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
        DelegationCommands::Delegate {
            peer,
            prompt,
            plan_id,
            api_url: delegate_api_url,
        } => {
            let url = format!("{delegate_api_url}/api/mesh/delegate");
            let body = serde_json::json!({
                "peer": peer,
                "prompt": prompt,
                "plan_id": plan_id,
            });
            let result = cli_http::post_and_return(&url, &body)
                .await
                .map_err(api_err)?;
            let delegation_id = result["delegation_id"].as_str().unwrap_or("-");
            let stream_url = result["stream_url"].as_str().unwrap_or("-");
            println!("Delegated to {peer}");
            println!("  delegation_id : {delegation_id}");
            println!("  stream_url    : {stream_url}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delegate_variant_parses_required_fields() {
        let cmd = DelegationCommands::Delegate {
            peer: "macProM1".to_string(),
            prompt: "test".to_string(),
            plan_id: None,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, DelegationCommands::Delegate { .. }));
    }

    #[test]
    fn delegate_variant_accepts_plan_id() {
        let cmd = DelegationCommands::Delegate {
            peer: "macProM1".to_string(),
            prompt: "run wave 1".to_string(),
            plan_id: Some(720),
            api_url: "http://localhost:8420".to_string(),
        };
        if let DelegationCommands::Delegate { peer, plan_id, .. } = cmd {
            assert_eq!(peer, "macProM1");
            assert_eq!(plan_id, Some(720));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn delegate_api_url_shape() {
        let api_url = "http://localhost:8420";
        let url = format!("{api_url}/api/mesh/delegate");
        assert_eq!(url, "http://localhost:8420/api/mesh/delegate");
    }

    #[test]
    fn delegate_body_contains_required_fields() {
        let peer = "macProM1";
        let prompt = "run plan 720 wave 1";
        let plan_id: Option<i64> = Some(720);
        let body = serde_json::json!({
            "peer": peer,
            "prompt": prompt,
            "plan_id": plan_id,
        });
        assert_eq!(body["peer"], "macProM1");
        assert_eq!(body["prompt"], "run plan 720 wave 1");
        assert_eq!(body["plan_id"], 720);
    }

    #[test]
    fn delegate_body_plan_id_null_when_none() {
        let body = serde_json::json!({
            "peer": "linux-worker",
            "prompt": "test task",
            "plan_id": serde_json::Value::Null,
        });
        assert!(body["plan_id"].is_null());
    }

    #[test]
    fn status_variant_still_exists() {
        let cmd = DelegationCommands::Status { plan_id: 42 };
        assert!(matches!(cmd, DelegationCommands::Status { plan_id: 42 }));
    }

    #[test]
    fn list_variant_still_exists() {
        let cmd = DelegationCommands::List;
        assert!(matches!(cmd, DelegationCommands::List));
    }
}
