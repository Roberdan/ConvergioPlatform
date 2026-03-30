use crate::cli_commands::Commands;
use crate::{
    cli_agent_format, cli_audit, cli_audit_project, cli_bus, cli_checkpoint, cli_delegation,
    cli_domain,
    cli_error::CliError,
    cli_capability, cli_channel, cli_chat, cli_kb, cli_kernel, cli_lock, cli_memory, cli_ops, cli_status, cli_voice, cli_plan, cli_project, cli_reap, cli_repo, cli_review, cli_run,
    cli_skill, cli_task, cli_wave, cli_who, cli_workspace,
    ipc_handler::{self, DaemonCommands},
};
use std::env;
use std::io::Read;
use std::process::ExitCode;

pub(crate) async fn dispatch(command: Commands) -> ExitCode {
    match command {
        Commands::Db {
            db_path,
            crsqlite_path,
            args,
        } => {
            let path = db_path.unwrap_or_else(ipc_handler::default_db_path);
            drop(crsqlite_path); // crsqlite removed; sync via timestamp adapter
            let db = match convergio_core::db::PlanDb::open_path(&path) {
                Ok(db) => db,
                Err(err) => {
                    eprintln!("db open failed: {err}");
                    return ExitCode::from(2);
                }
            };
            if let Err(e) = convergio_core::db::migrations::run(db.connection()) {
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
                    return ExitCode::from(2);
                }
            }
            ExitCode::SUCCESS
        }
        Commands::Hook { mode } => {
            let mut input = String::new();
            if std::io::stdin().read_to_string(&mut input).is_err() || input.trim().is_empty() {
                return ExitCode::SUCCESS;
            }
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let context = convergio_core::hooks::checks::CheckContext::from_env(&home);
            if mode == "pre" {
                match convergio_core::hooks::dispatch_pre_tool(&input, &context) {
                    Ok(Some(result)) => println!("{result}"),
                    Ok(None) => {}
                    Err(err) => {
                        eprintln!("{err}");
                        return ExitCode::from(1);
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Commands::Serve {
            bind,
            static_dir,
            crsqlite_path,
            db_path,
            dev_mode,
            mesh,
        } => {
            if let Some(ref p) = db_path {
                std::env::set_var("DASHBOARD_DB", p);
            }
            // crsqlite removed — sync via timestamp-based adapter (libsql_adapter)
            drop(crsqlite_path);
            let db_path = ipc_handler::default_db_path();
            tokio::spawn(convergio_core::background::run_pause_bridge(db_path));
            if let Err(e) =
                ipc_handler::run_serve(bind, static_dir, None, dev_mode, mesh).await
            {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
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
                // HTTP LWW sync re-enabled as primary replication path.
                // CRDT over TCP (crsqlite) is optional enhancement — not compiled/deployed on nodes.
                let resolved_db = db_path.clone().unwrap_or_else(ipc_handler::default_db_path);
                if let Ok(sync_conn) = rusqlite::Connection::open(&resolved_db) {
                    let sync_conn = std::sync::Arc::new(std::sync::Mutex::new(sync_conn));
                    let sync_interval = convergio_core::background_sync::resolve_interval_secs(None);
                    let _sync_handle =
                        convergio_core::background_sync::spawn_sync_loop(sync_conn, sync_interval);
                }

                if let Err(e) = ipc_handler::run_daemon(
                    bind_ip,
                    port,
                    peers_conf,
                    db_path,
                    crsqlite_path,
                    local_only,
                )
                .await
                {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
                ExitCode::SUCCESS
            }
        },
        Commands::Ipc { args } => {
            if let Err(e) = convergio_core::ipc::cli::run_ipc(args).await {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Commands::IpcIntel { command } => {
            if let Err(e) = ipc_handler::handle_ipc(command).await {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Commands::Tui { api_url } => {
            env::set_var("CONVERGIO_API_URL", &api_url);
            match convergio_core::tui::TuiApp::new() {
                Ok(mut app) => {
                    if let Err(err) = app.run().await {
                        eprintln!("TUI error: {err}");
                        return ExitCode::from(2);
                    }
                }
                Err(err) => {
                    eprintln!("TUI init failed: {err}");
                    return ExitCode::from(2);
                }
            }
            ExitCode::SUCCESS
        }
        Commands::Plan { command } => exit_on_err(cli_plan::handle(command).await),
        Commands::Task { command } => exit_on_err(cli_task::handle(command).await),
        Commands::Wave { command } => exit_on_err(cli_wave::handle(command).await),
        Commands::Agent { command } => exit_on_err(cli_agent_format::dispatch(command).await),
        Commands::Kb { command } => exit_on_err(cli_kb::handle(command).await),
        Commands::Status { api_url } => exit_on_err(cli_status::handle(&api_url).await),
        Commands::Chat { message, api_url } => exit_on_err(cli_chat::handle(&api_url, message).await),
        Commands::Channel { command } => exit_on_err(cli_channel::handle(command).await),
        Commands::Capability { command } => exit_on_err(cli_capability::handle(command).await),
        Commands::Voice { command } => exit_on_err(cli_voice::handle(command).await),
        Commands::Memory { command } => exit_on_err(cli_memory::handle(command).await),
        Commands::Run { command } => exit_on_err(cli_run::handle(command).await),
        Commands::Mesh { command } => {
            cli_ops::handle_mesh(command).await;
            ExitCode::SUCCESS
        }
        Commands::Session { command } => {
            cli_ops::handle_session(command).await;
            ExitCode::SUCCESS
        }
        Commands::Checkpoint { command } => {
            cli_checkpoint::handle(command).await;
            ExitCode::SUCCESS
        }
        Commands::Lock { command } => {
            cli_lock::handle(command).await;
            ExitCode::SUCCESS
        }
        Commands::Review { command } => {
            cli_review::handle(command).await;
            ExitCode::SUCCESS
        }
        Commands::Audit {
            path,
            project,
            output,
            yes,
            api_url,
        } => {
            if let Some(project_id) = project {
                exit_on_err(cli_audit_project::handle(&project_id, output, yes, &api_url).await)
            } else {
                exit_on_err(cli_audit::handle(path))
            }
        }
        Commands::Skill { command } => exit_on_err(cli_skill::handle(command).await),
        Commands::Bus { command } => {
            cli_bus::handle(command).await;
            ExitCode::SUCCESS
        }
        Commands::Project { command } => exit_on_err(cli_project::handle(command).await),
        Commands::Metrics { command } => {
            cli_ops::handle_metrics(command).await;
            ExitCode::SUCCESS
        }
        Commands::Alert { command } => {
            cli_ops::handle_alert(command).await;
            ExitCode::SUCCESS
        }
        Commands::Domain { command } => exit_on_err(cli_domain::dispatch(command).await),
        Commands::Workspace { command } => {
            cli_workspace::handle(command).await;
            ExitCode::SUCCESS
        }
        Commands::Who { command } => {
            exit_on_err(cli_who::handle(command, "http://localhost:8420").await)
        }
        Commands::Delegation { command } => {
            exit_on_err(cli_delegation::handle(command, "http://localhost:8420").await)
        }
        Commands::Reap { command } => {
            cli_reap::handle(command).await;
            ExitCode::SUCCESS
        }
        Commands::Repo { command } => exit_on_err(cli_repo::handle(command).await),
        Commands::Kernel { command } => cli_kernel::dispatch(command).await,
        Commands::Cheatsheet => { crate::cli_cheatsheet::print_cheatsheet(); ExitCode::SUCCESS }
        Commands::Api => { crate::cli_api_list::print_api_list(); ExitCode::SUCCESS }
    }
}
fn exit_on_err(result: Result<(), CliError>) -> ExitCode {
    if let Err(e) = result {
        eprintln!("{e}");
        return ExitCode::from(e.exit_code() as u8);
    }
    ExitCode::SUCCESS
}
