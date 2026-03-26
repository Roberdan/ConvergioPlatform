// cvg delegation — manage plan delegation to mesh workers.
// Why: Plan 706 — no orchestration for remote execution. Zero traceability.

use crate::cli_error::CliError;
use crate::cli_http;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum DelegationCommands {
    /// Show delegation status for a plan (peer, status, task, last update, output)
    Status {
        /// Plan ID to inspect (alias: --plan)
        #[arg(long, short = 'p')]
        plan: Option<i64>,
        /// Positional plan_id (legacy; prefer --plan)
        plan_id: Option<i64>,
        /// Poll every 5s and refresh output
        #[arg(long)]
        live: bool,
    },
    /// List all active plans (potential delegations)
    List,
}

fn api_err(code: i32) -> CliError {
    CliError::ApiCallFailed(format!("daemon returned error (code {code})"))
}

/// Render a single delegation row (padded columns).
pub fn format_progress_row(d: &serde_json::Value) -> String {
    let peer = d["peer"].as_str().unwrap_or("-");
    let status = d["status"].as_str().unwrap_or("-");
    let task = d["current_task"].as_str().unwrap_or("-");
    let updated = d["last_update"].as_str().unwrap_or("-");
    let summary = d["output_summary"].as_str().unwrap_or("");
    // Truncate long fields to fit 100-char terminal width
    let task_short = if task.len() > 20 { &task[..20] } else { task };
    let summary_short = if summary.len() > 40 {
        &summary[..40]
    } else {
        summary
    };
    format!(
        "{:<20} {:<10} {:<22} {:<24} {}",
        peer, status, task_short, updated, summary_short
    )
}

/// Render the column header line.
pub fn format_progress_header() -> String {
    format!(
        "{:<20} {:<10} {:<22} {:<24} {}",
        "PEER", "STATUS", "TASK", "LAST UPDATE", "OUTPUT"
    )
}

/// Render a full progress table from the API response.
pub fn format_progress_table(body: &serde_json::Value) -> String {
    let list = match body["delegations"].as_array() {
        Some(arr) if !arr.is_empty() => arr,
        _ => return "No active delegations.".to_string(),
    };
    let sep = "-".repeat(100);
    let mut lines = vec![format_progress_header(), sep];
    for d in list {
        lines.push(format_progress_row(d));
    }
    lines.join("\n")
}

async fn fetch_progress(api_url: &str, plan_id: i64) -> Result<serde_json::Value, CliError> {
    let url = format!("{api_url}/api/delegation/{plan_id}/progress");
    cli_http::get_and_return(&url).await.map_err(api_err)
}

pub async fn handle(cmd: DelegationCommands, api_url: &str) -> Result<(), CliError> {
    match cmd {
        DelegationCommands::Status {
            plan,
            plan_id,
            live,
        } => {
            let id = plan.or(plan_id).ok_or_else(|| {
                CliError::ApiCallFailed("provide --plan <ID> or positional plan_id".to_string())
            })?;

            if live {
                // Poll every 5s until interrupted
                loop {
                    // Clear screen with ANSI escape for live refresh
                    print!("\x1B[2J\x1B[H");
                    let body = fetch_progress(api_url, id).await?;
                    println!("Plan {id} — delegation status (live, Ctrl+C to exit)");
                    println!("{}", format_progress_table(&body));
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            } else {
                let body = fetch_progress(api_url, id).await?;
                println!("Plan {id} — delegation status");
                println!("{}", format_progress_table(&body));
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
    }
}

#[cfg(test)]
#[path = "cli_delegation_tests.rs"]
mod tests;
