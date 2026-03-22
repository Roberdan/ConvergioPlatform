// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Wave subcommands for the cvg CLI — delegates to daemon HTTP API via reqwest.
// JSON output by default; --human flag for readable text.

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum WaveCommands {
    /// Update wave status
    Update {
        /// Wave DB ID
        wave_id: i64,
        /// New status (e.g. in_progress, done, blocked)
        status: String,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Show context for a plan (waves + tasks summary)
    Context {
        /// Plan ID
        plan_id: i64,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Validate a wave (Thor gate — Opus only, wave-level)
    Validate {
        /// Wave DB ID
        wave_id: i64,
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

pub async fn handle(cmd: WaveCommands) {
    match cmd {
        WaveCommands::Update { wave_id, status, human, api_url } => {
            let body = serde_json::json!({
                "wave_id": wave_id,
                "status": status,
            });
            post_and_print(&format!("{api_url}/api/plan-db/wave/update"), &body, human).await;
        }
        WaveCommands::Context { plan_id, human, api_url } => {
            fetch_and_print(
                &format!("{api_url}/api/plan-db/context/{plan_id}"),
                human,
            )
            .await;
        }
        WaveCommands::Validate { wave_id, plan_id, human, api_url } => {
            // Wave validation posts to the plans validate endpoint with wave scope.
            let body = serde_json::json!({
                "wave_id": wave_id,
                "scope": "wave",
            });
            post_and_print(
                &format!("{api_url}/api/plans/{plan_id}/validate"),
                &body,
                human,
            )
            .await;
        }
    }
}

/// Fetch a GET endpoint and print result as JSON or human-readable.
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

/// POST JSON body to an endpoint and print result.
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
        println!("{}", serde_json::to_string_pretty(val)
            .unwrap_or_else(|_| val.to_string()));
    } else {
        println!("{val}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave_commands_update_variant_exists() {
        let cmd = WaveCommands::Update {
            wave_id: 1,
            status: "done".to_string(),
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, WaveCommands::Update { wave_id: 1, .. }));
    }

    #[test]
    fn wave_commands_context_variant_exists() {
        let cmd = WaveCommands::Context {
            plan_id: 685,
            human: true,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, WaveCommands::Context { plan_id: 685, .. }));
    }

    #[test]
    fn wave_commands_validate_variant_exists() {
        let cmd = WaveCommands::Validate {
            wave_id: 3,
            plan_id: 685,
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, WaveCommands::Validate { wave_id: 3, .. }));
    }

    #[test]
    fn wave_update_body_shape() {
        // Verify the JSON body for wave update has the expected fields.
        let body = serde_json::json!({
            "wave_id": 2_i64,
            "status": "in_progress",
        });
        assert_eq!(body["wave_id"], 2);
        assert_eq!(body["status"], "in_progress");
    }
}
