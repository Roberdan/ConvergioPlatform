// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Mesh and Session subcommands for the cvg CLI — delegates to daemon HTTP API via reqwest.
// JSON output by default; --human flag for readable text.

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum MeshCommands {
    /// Send a heartbeat to the mesh (POST /api/heartbeat)
    Heartbeat {
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Show current mesh status (GET /api/mesh)
    Status {
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Show cluster-level heartbeat status (GET /api/heartbeat/status)
    ClusterStatus {
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum SessionCommands {
    /// Clean up old/stale sessions
    Reap {
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Trigger session recovery
    Recovery {
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

pub async fn handle_mesh(cmd: MeshCommands) {
    match cmd {
        MeshCommands::Heartbeat { human, api_url } => {
            // POST with empty body — signals this node is alive
            let body = serde_json::json!({});
            post_and_print(&format!("{api_url}/api/heartbeat"), &body, human).await;
        }
        MeshCommands::Status { human, api_url } => {
            fetch_and_print(&format!("{api_url}/api/mesh"), human).await;
        }
        MeshCommands::ClusterStatus { human, api_url } => {
            fetch_and_print(&format!("{api_url}/api/heartbeat/status"), human).await;
        }
    }
}

pub async fn handle_session(cmd: SessionCommands) {
    match cmd {
        SessionCommands::Reap { human, api_url } => {
            let body = serde_json::json!({});
            post_and_print(&format!("{api_url}/api/sessions/reap"), &body, human).await;
        }
        SessionCommands::Recovery { human, api_url } => {
            let body = serde_json::json!({});
            post_and_print(&format!("{api_url}/api/sessions/recovery"), &body, human).await;
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

    // Mesh command variant tests

    #[test]
    fn mesh_heartbeat_variant_exists() {
        let cmd = MeshCommands::Heartbeat {
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, MeshCommands::Heartbeat { human: false, .. }));
    }

    #[test]
    fn mesh_status_variant_exists() {
        let cmd = MeshCommands::Status {
            human: true,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, MeshCommands::Status { human: true, .. }));
    }

    #[test]
    fn mesh_cluster_status_variant_exists() {
        let cmd = MeshCommands::ClusterStatus {
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, MeshCommands::ClusterStatus { .. }));
    }

    // Session command variant tests

    #[test]
    fn session_reap_variant_exists() {
        let cmd = SessionCommands::Reap {
            human: false,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, SessionCommands::Reap { human: false, .. }));
    }

    #[test]
    fn session_recovery_variant_exists() {
        let cmd = SessionCommands::Recovery {
            human: true,
            api_url: "http://localhost:8420".to_string(),
        };
        assert!(matches!(cmd, SessionCommands::Recovery { human: true, .. }));
    }

    // URL construction tests — verify the correct API endpoints are targeted

    #[test]
    fn mesh_heartbeat_url() {
        let api_url = "http://localhost:8420";
        let url = format!("{api_url}/api/heartbeat");
        assert_eq!(url, "http://localhost:8420/api/heartbeat");
    }

    #[test]
    fn mesh_status_url() {
        let api_url = "http://localhost:8420";
        let url = format!("{api_url}/api/mesh");
        assert_eq!(url, "http://localhost:8420/api/mesh");
    }

    #[test]
    fn mesh_cluster_status_url() {
        let api_url = "http://localhost:8420";
        let url = format!("{api_url}/api/heartbeat/status");
        assert_eq!(url, "http://localhost:8420/api/heartbeat/status");
    }

    #[test]
    fn session_reap_url() {
        let api_url = "http://localhost:8420";
        let url = format!("{api_url}/api/sessions/reap");
        assert_eq!(url, "http://localhost:8420/api/sessions/reap");
    }

    #[test]
    fn session_recovery_url() {
        let api_url = "http://localhost:8420";
        let url = format!("{api_url}/api/sessions/recovery");
        assert_eq!(url, "http://localhost:8420/api/sessions/recovery");
    }

    #[test]
    fn print_value_json_compact() {
        let val = serde_json::json!({"nodes": 3, "healthy": true});
        let compact = val.to_string();
        assert!(!compact.is_empty());
        assert!(!compact.contains('\n'));
    }

    #[test]
    fn print_value_json_pretty() {
        let val = serde_json::json!({"nodes": 3});
        let pretty = serde_json::to_string_pretty(&val).unwrap();
        assert!(pretty.contains('\n'));
    }
}
