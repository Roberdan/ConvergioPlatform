// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// CLI commands for the kernel health monitor (cvg kernel start/stop/status).

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum KernelCommands {
    /// Start the kernel engine (loads default model, enables /api/kernel/* endpoints)
    Start {
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
        /// Override MLX model to load
        #[arg(long)]
        model: Option<String>,
    },
    /// Stop the kernel engine
    Stop {
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Show kernel status
    Status {
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

/// Dispatch kernel subcommands. Returns an exit code (0 = success).
pub async fn dispatch(cmd: KernelCommands) -> std::process::ExitCode {
    match cmd {
        KernelCommands::Status { api_url } => {
            let url = format!("{api_url}/api/kernel/status");
            match reqwest::get(&url).await {
                Ok(resp) => {
                    let text = resp.text().await.unwrap_or_default();
                    println!("{text}");
                    std::process::ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("kernel status error: {e}");
                    std::process::ExitCode::FAILURE
                }
            }
        }
        KernelCommands::Start { api_url, model } => {
            println!(
                "kernel engine is embedded — restart daemon with --kernel flag to enable. \
                 api_url={api_url} model={model:?}"
            );
            std::process::ExitCode::SUCCESS
        }
        KernelCommands::Stop { api_url } => {
            println!("kernel stop: restart daemon without --kernel flag. api_url={api_url}");
            std::process::ExitCode::SUCCESS
        }
    }
}
