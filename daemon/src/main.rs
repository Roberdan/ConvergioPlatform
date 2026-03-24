mod daemon_logging;

mod cli_agent;
mod cli_agent_format;
mod cli_agents;
mod cli_audit;
mod cli_audit_project;
mod cli_bus;
mod cli_checkpoint;
mod cli_commands;
mod cli_delegation;
mod cli_domain;
mod cli_error;
mod cli_http;
mod cli_kb;
mod cli_lock;
mod cli_ops;
mod cli_plan;
mod cli_plan_handlers;
mod cli_project;
mod cli_review;
mod cli_run;
mod cli_skill;
mod cli_skill_disable;
mod cli_skill_enable;
mod cli_skill_transpile;
mod cli_skill_validate;
mod cli_skill_validators;
mod cli_task;
mod cli_task_approve;
mod cli_wave;
mod cli_wave_handlers;
mod cli_who;
mod cli_workspace;
mod ipc_handler;
mod main_dispatch;
mod message_error;
mod transpiler;

use clap::Parser;
use cli_commands::Commands;
use std::env;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(
    name = "claude-core",
    version,
    about = "Core runtime for Claude utilities"
)]
struct Cli {
    #[arg(long, default_value_t = false)]
    version_json: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize logging + panic hook before anything else.
    // TUI mode uses file-only logging to avoid corrupting ratatui display.
    let is_tui = env::args().any(|a| a == "tui");
    let _log_guard = if is_tui {
        daemon_logging::init_file_only()
    } else {
        daemon_logging::init()
    };

    // argv[0] detection: agent-ipc symlink routes to `ipc`, cvg symlink routes to CLI
    let args: Vec<String> = env::args().collect();
    let cli = if args
        .first()
        .map(|a| a.contains("agent-ipc"))
        .unwrap_or(false)
    {
        let mut new_args = vec![args[0].clone(), "ipc".to_string()];
        new_args.extend(args[1..].to_vec());
        Cli::parse_from(new_args)
    } else {
        Cli::parse()
    };

    if cli.version_json {
        let payload = serde_json::json!({
            "binary": "claude-core",
            "version": env!("CARGO_PKG_VERSION")
        });
        println!("{payload}");
        return ExitCode::SUCCESS;
    }
    if let Some(command) = cli.command {
        return main_dispatch::dispatch(command).await;
    }
    println!("claude-core scaffold ready");
    ExitCode::SUCCESS
}
