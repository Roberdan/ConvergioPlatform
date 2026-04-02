mod daemon_logging;

mod cli_agent;
mod cli_agent_format;
mod cli_agent_history;
mod cli_ask;
mod cli_capability;
mod cli_channel;
mod cli_chat;
mod cli_cheatsheet;
mod cli_status;
mod cli_kernel;
mod cli_agents;
mod cli_api_list;
mod cli_audit;
mod cli_audit_project;
mod cli_create_org;
mod cli_bus;
mod cli_bus_org;
mod cli_bus_ask;
mod cli_bus_watch;
mod cli_checkpoint;
mod cli_commands;
mod cli_delegation;
mod cli_setup;
mod cli_setup_steps;
mod cli_domain;
mod cli_error;
mod cli_http;
mod cli_kb;
mod cli_launch;
mod cli_lock;
mod cli_mesh_join;
mod cli_memory;
mod cli_ops;
mod cli_org;
mod cli_org_show;
mod cli_plan;
mod cli_plan_handlers;
mod cli_plan_show;
mod cli_plan_template;
mod cli_plan_tree_fmt;
mod cli_project;
mod cli_project_tree;
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
mod cli_task_format;
mod cli_wave;
mod cli_wave_handlers;
mod cli_reap;
mod cli_repo;
mod cli_voice;
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
    name = "cvg",
    version,
    about = "Convergio Platform CLI -- orchestrate agents, plans, and infrastructure",
    long_about = "Convergio Platform CLI -- orchestrate agents, plans, and infrastructure\n\n\
        Unified command-line interface for the Convergio agentic platform.\n\
        Manages plans, tasks, agents, organizations, mesh networking, and more.\n\n\
        Quick start:\n  cvg status              # Platform overview\n  \
        cvg plan list             # List execution plans\n  \
        cvg who agents            # See active agents\n  \
        cvg org list              # List organizations\n  \
        cvg cheatsheet            # Full command reference",
)]
struct Cli {
    #[arg(long, default_value_t = false)]
    version_json: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() -> ExitCode {
    // Load ~/.convergio/env so CLI commands (cvg) pick up auth tokens
    // without requiring the user to source the file in their shell profile.
    if let Ok(contents) = std::fs::read_to_string(
        dirs::home_dir()
            .unwrap_or_default()
            .join(".convergio/env"),
    ) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                if env::var(k.trim()).is_err() {
                    env::set_var(k.trim(), v.trim());
                }
            }
        }
    }

    // Load config.toml (falls back to defaults if missing).
    let _cfg = convergio_core::config::load_config();
    let cfg_issues = convergio_core::config::validation::validate(&_cfg);
    for issue in &cfg_issues {
        eprintln!("[config] warning: {issue}");
    }

    // Wrap config in Arc<RwLock> for hot-reload and spawn the watcher.
    let shared_cfg = std::sync::Arc::new(std::sync::RwLock::new(_cfg));
    if let Err(e) = convergio_core::config::watcher::spawn_config_watcher(
        std::sync::Arc::clone(&shared_cfg),
    ) {
        eprintln!("[config] watcher not started: {e}");
    }

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
    if let Some(ref command) = cli.command {
        // First-run hint: suggest `cvg setup` if no config exists yet.
        if !matches!(command, Commands::Setup { .. })
            && !convergio_core::config::config_path().exists()
        {
            eprintln!("hint: First time? Run `cvg setup` to configure Convergio.");
        }
    }
    if let Some(command) = cli.command {
        return main_dispatch::dispatch(command).await;
    }
    println!("claude-core scaffold ready");
    ExitCode::SUCCESS
}
