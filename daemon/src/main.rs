mod cli_agent;
mod cli_agents;
mod cli_audit;
mod cli_checkpoint;
mod cli_commands;
mod cli_http;
mod cli_kb;
mod cli_lock;
mod cli_ops;
mod cli_plan;
mod cli_plan_handlers;
mod cli_review;
mod cli_run;
mod cli_skill;
mod cli_skill_transpile;
mod cli_task;
mod cli_wave;
mod ipc_handler;

use clap::Parser;
use cli_commands::Commands;
use ipc_handler::DaemonCommands;
use std::env;
use std::io::Read;

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
async fn main() {
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
    } else if args
        .first()
        .map(|a| {
            let base = std::path::Path::new(a)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            base == "cvg"
        })
        .unwrap_or(false)
    {
        Cli::parse()
    } else {
        Cli::parse()
    };

    if cli.version_json {
        let payload = serde_json::json!({
            "binary": "claude-core",
            "version": env!("CARGO_PKG_VERSION")
        });
        println!("{payload}");
        return;
    }
    if let Some(command) = cli.command {
        dispatch(command).await;
        return;
    }
    println!("claude-core scaffold ready");
}

async fn dispatch(command: Commands) {
    match command {
        Commands::Db {
            db_path,
            crsqlite_path,
            args,
        } => {
            let path = db_path.unwrap_or_else(ipc_handler::default_db_path);
            let db = match claude_core::db::PlanDb::open_path(&path, crsqlite_path) {
                Ok(db) => db,
                Err(err) => {
                    eprintln!("db open failed: {err}");
                    std::process::exit(2);
                }
            };
            if let Err(e) = claude_core::db::migrations::run(db.connection()) {
                eprintln!("[startup] migrations failed: {e}");
            }
            let command = args.first().map(String::as_str).unwrap_or_default();
            let mut stdin_payload = None;
            if command == "apply-changes" {
                let mut buf = String::new();
                if std::io::stdin().read_to_string(&mut buf).is_ok() {
                    stdin_payload = Some(buf);
                }
            }
            match db.run_subcommand_with_input(&args, stdin_payload.as_deref()) {
                Ok(output) => println!("{output}"),
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(2);
                }
            }
        }
        Commands::Hook { mode } => {
            let mut input = String::new();
            if std::io::stdin().read_to_string(&mut input).is_err() || input.trim().is_empty() {
                return;
            }
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let context = claude_core::hooks::checks::CheckContext::from_env(&home);
            if mode == "pre" {
                match claude_core::hooks::dispatch_pre_tool(&input, &context) {
                    Ok(Some(result)) => println!("{result}"),
                    Ok(None) => {}
                    Err(err) => {
                        eprintln!("{err}");
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Serve {
            bind,
            static_dir,
            crsqlite_path,
        } => {
            let db_path = ipc_handler::default_db_path();
            tokio::spawn(claude_core::background::run_pause_bridge(db_path));
            ipc_handler::run_serve(bind, static_dir, crsqlite_path).await;
        }
        Commands::Daemon { command } => match command {
            DaemonCommands::Start {
                bind_ip,
                port,
                peers_conf,
                db_path,
                crsqlite_path,
                local_only,
            } => {
                ipc_handler::run_daemon(
                    bind_ip, port, peers_conf, db_path, crsqlite_path, local_only,
                )
                .await;
            }
        },
        Commands::Ipc { args } => {
            if let Err(e) = claude_core::ipc::cli::run_ipc(args).await {
                eprintln!("{e}");
                std::process::exit(2);
            }
        }
        Commands::IpcIntel { command } => {
            ipc_handler::handle_ipc(command).await;
        }
        Commands::Tui { api_url } => {
            env::set_var("CONVERGIO_API_URL", &api_url);
            match claude_core::tui::TuiApp::new() {
                Ok(mut app) => {
                    if let Err(err) = app.run().await {
                        eprintln!("TUI error: {err}");
                        std::process::exit(2);
                    }
                }
                Err(err) => {
                    eprintln!("TUI init failed: {err}");
                    std::process::exit(2);
                }
            }
        }
        Commands::Plan { command } => cli_plan::handle(command).await,
        Commands::Task { command } => cli_task::handle(command).await,
        Commands::Wave { command } => cli_wave::handle(command).await,
        Commands::Agent { command } => cli_agent::handle(command).await,
        Commands::Kb { command } => cli_kb::handle(command).await,
        Commands::Run { command } => cli_run::handle(command).await,
        Commands::Mesh { command } => cli_ops::handle_mesh(command).await,
        Commands::Session { command } => cli_ops::handle_session(command).await,
        Commands::Checkpoint { command } => cli_checkpoint::handle(command).await,
        Commands::Lock { command } => cli_lock::handle(command).await,
        Commands::Review { command } => cli_review::handle(command).await,
        Commands::Audit { path } => cli_audit::handle(path),
        Commands::Skill { command } => cli_skill::handle(command).await,
    }
}
