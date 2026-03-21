use clap::Subcommand;
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Debug, Subcommand)]
pub enum IpcCommands {
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    Models {
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Sub {
        #[command(subcommand)]
        command: SubCommands,
    },
    Budget {
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Route {
        task_description: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        parallel: bool,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Skills {
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    RequestSkill {
        skill: String,
        #[arg(long)]
        payload: String,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    RespondSkill {
        request_id: String,
        #[arg(long)]
        result: String,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    RateSkill {
        request_id: String,
        rating: f64,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCommands {
    Store {
        service: String,
        token: String,
        #[arg(long, env = "IPC_SHARED_SECRET")]
        secret: String,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Get {
        service: String,
        #[arg(long, env = "IPC_SHARED_SECRET")]
        secret: String,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Revoke {
        service: String,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Rotate {
        #[arg(long, env = "IPC_OLD_SECRET")]
        old_secret: String,
        #[arg(long, env = "IPC_NEW_SECRET")]
        new_secret: String,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum SubCommands {
    Add {
        name: String,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        plan: String,
        #[arg(long)]
        budget: f64,
        #[arg(long, default_value_t = 1)]
        reset_day: i32,
        #[arg(long, value_delimiter = ',')]
        models: Vec<String>,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Remove {
        name: String,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
}

pub async fn handle_ipc(command: IpcCommands) {
    match command {
        IpcCommands::Auth { command } => handle_auth(command).await,
        IpcCommands::Models { db_path } => handle_models(db_path),
        IpcCommands::Sub { command } => handle_sub(command),
        IpcCommands::Budget { db_path } => handle_budget(db_path),
        IpcCommands::Route {
            task_description,
            dry_run,
            parallel,
            db_path,
        } => handle_route(task_description, dry_run, parallel, db_path),
        IpcCommands::Skills { agent, db_path } => handle_skills(agent, db_path),
        IpcCommands::RequestSkill {
            skill,
            payload,
            db_path,
        } => handle_request_skill(skill, payload, db_path),
        IpcCommands::RespondSkill {
            request_id,
            result,
            db_path,
        } => handle_respond_skill(request_id, result, db_path),
        IpcCommands::RateSkill {
            request_id,
            rating,
            db_path,
        } => handle_rate_skill(request_id, rating, db_path),
    }
}

pub fn default_db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude/data/dashboard.db")}

pub fn default_peers_conf() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude/config/peers.conf")
}

#[derive(Debug, clap::Subcommand)]
pub enum DaemonCommands {
    Start {
        #[arg(long)]
        bind_ip: Option<String>,
        #[arg(long, default_value_t = 9420)]
        port: u16,
        #[arg(long)]
        peers_conf: Option<PathBuf>,
        #[arg(long)]
        db_path: Option<PathBuf>,
        #[arg(long)]
        crsqlite_path: Option<String>,
        #[arg(long, default_value_t = false)]
        local_only: bool,
    },
}

pub async fn run_serve(
    bind: String,
    static_dir: Option<PathBuf>,
    crsqlite_path: Option<String>,
) {
    // Init structured logging to file + stderr
    let log_dir = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".claude/logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let file_appender = tracing_appender::rolling::daily(&log_dir, "claude-core.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "claude_core=info,tower_http=info".parse().unwrap()),
        )
        .with_writer(non_blocking)
        .with_ansi(false)
        .compact()
        .init();
    let dir = static_dir.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        claude_core::server::resolve_dashboard_static_dir(PathBuf::from(home).join(".claude"))
    });
    info!("claude-core serve → {bind} (static: {dir:?})");
    eprintln!("claude-core serve → {bind} (static: {dir:?})");
    if let Err(err) = claude_core::server::run(&bind, dir, crsqlite_path).await {
        warn!("server failed: {err}");
        eprintln!("server failed: {err}");
        std::process::exit(2);
    }
}

pub async fn run_daemon(
    bind_ip: Option<String>,
    port: u16,
    peers_conf: Option<PathBuf>,
    db_path: Option<PathBuf>,
    crsqlite_path: Option<String>,
    local_only: bool,
) {
    let resolved_ip = if local_only {
        bind_ip.unwrap_or_else(|| "127.0.0.1".to_string())
    } else {
        bind_ip
            .or_else(|| std::env::var("TAILSCALE_IP").ok())
            .or_else(claude_core::mesh::daemon::detect_tailscale_ip)
            .unwrap_or_else(|| "0.0.0.0".to_string())
    };
    let config = claude_core::mesh::daemon::DaemonConfig {
        bind_ip: resolved_ip,
        port,
        peers_conf_path: peers_conf.unwrap_or_else(default_peers_conf),
        db_path: db_path.unwrap_or_else(default_db_path),
        crsqlite_path,
        local_only,
    };
    if let Err(err) = claude_core::mesh::daemon::run_service(config).await {
        eprintln!("daemon start failed: {err}");
        std::process::exit(2);
    }
}

async fn handle_auth(command: AuthCommands) {
    let db_path = match &command {
        AuthCommands::Store { db_path, .. }
        | AuthCommands::List { db_path }
        | AuthCommands::Get { db_path, .. }
        | AuthCommands::Revoke { db_path, .. }
        | AuthCommands::Rotate { db_path, .. } => db_path.clone().unwrap_or_else(default_db_path),
    };
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open failed: {e}");
            std::process::exit(2);
        }
    };
    match command {
        AuthCommands::Store {
            service,
            token,
            secret,
            ..
        } => {
            match claude_core::ipc::auth_sync::store_token(&conn, &service, &token, &secret) {
                Ok(()) => println!("stored token for {service}"),
                Err(e) => {
                    eprintln!("store failed: {e}");
                    std::process::exit(2);
                }
            }
        }
        AuthCommands::List { .. } => {
            match claude_core::ipc::auth_sync::list_tokens(&conn) {
                Ok(tokens) => {
                    println!("{:<20} {:<20} {}", "SERVICE", "HOST", "UPDATED");
                    for t in &tokens {
                        println!("{:<20} {:<20} {}", t.service, t.host, t.updated_at);
                    }
                    println!("\n{} token(s)", tokens.len());
                }
                Err(e) => {
                    eprintln!("list failed: {e}");
                    std::process::exit(2);
                }
            }
        }
        AuthCommands::Get {
            service, secret, ..
        } => match claude_core::ipc::auth_sync::get_token(&conn, &service, &secret) {
            Ok(Some(val)) => println!("{val}"),
            Ok(None) => {
                eprintln!("no token found for {service}");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("get failed: {e}");
                std::process::exit(2);
            }
        },
        AuthCommands::Revoke { service, host, .. } => {
            let h = host.unwrap_or_else(|| {
                hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "unknown".to_string())
            });
            match claude_core::ipc::auth_sync::revoke_token(&conn, &service, &h) {
                Ok(n) => println!("revoked {n} token(s) for {service}@{h}"),
                Err(e) => {
                    eprintln!("revoke failed: {e}");
                    std::process::exit(2);
                }
            }
        }
        AuthCommands::Rotate {
            old_secret,
            new_secret,
            ..
        } => {
            match claude_core::ipc::auth_sync::rotate_keys(&conn, &old_secret, &new_secret) {
                Ok(n) => println!("rotated {n} token(s)"),
                Err(e) => {
                    eprintln!("rotate failed: {e}");
                    std::process::exit(2);
                }
            }
        }
    }
}

fn handle_models(db_path: Option<PathBuf>) {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = match rusqlite::Connection::open(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open: {e}");
            std::process::exit(2);
        }
    };
    match claude_core::ipc::models::get_all_models(&conn) {
        Ok(models) => {
            println!(
                "{:<15} {:<10} {:<30} {:>8} {:<10} {}",
                "HOST", "PROVIDER", "MODEL", "SIZE_GB", "QUANT", "LAST_SEEN"
            );
            for m in &models {
                println!(
                    "{:<15} {:<10} {:<30} {:>8.1} {:<10} {}",
                    m.host, m.provider, m.model, m.size_gb, m.quantization, m.last_seen
                );
            }
            println!("\n{} model(s)", models.len());
        }
        Err(e) => {
            eprintln!("list models: {e}");
            std::process::exit(2);
        }
    }
}

fn handle_sub(command: SubCommands) {
    let db_path = match &command {
        SubCommands::Add { db_path, .. }
        | SubCommands::List { db_path }
        | SubCommands::Remove { db_path, .. } => db_path.clone().unwrap_or_else(default_db_path),
    };
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open: {e}");
            std::process::exit(2);
        }
    };
    match command {
        SubCommands::Add {
            name,
            provider,
            plan,
            budget,
            reset_day,
            models,
            ..
        } => {
            let sub = claude_core::ipc::models::Subscription {
                name,
                provider,
                plan,
                budget_usd: budget,
                reset_day,
                models,
            };
            match claude_core::ipc::models::add_subscription(&conn, &sub) {
                Ok(()) => println!("added subscription {}", sub.name),
                Err(e) => {
                    eprintln!("add sub: {e}");
                    std::process::exit(2);
                }
            }
        }
        SubCommands::List { .. } => {
            match claude_core::ipc::models::list_subscriptions(&conn) {
                Ok(subs) => {
                    println!(
                        "{:<20} {:<12} {:<10} {:>10} {:>5} {}",
                        "NAME", "PROVIDER", "PLAN", "BUDGET", "DAY", "MODELS"
                    );
                    for s in &subs {
                        println!(
                            "{:<20} {:<12} {:<10} {:>10.2} {:>5} {}",
                            s.name,
                            s.provider,
                            s.plan,
                            s.budget_usd,
                            s.reset_day,
                            s.models.join(",")
                        );
                    }
                    println!("\n{} subscription(s)", subs.len());
                }
                Err(e) => {
                    eprintln!("list subs: {e}");
                    std::process::exit(2);
                }
            }
        }
        SubCommands::Remove { name, .. } => {
            match claude_core::ipc::models::remove_subscription(&conn, &name) {
                Ok(n) => println!("removed {n} subscription(s)"),
                Err(e) => {
                    eprintln!("remove sub: {e}");
                    std::process::exit(2);
                }
            }
        }
    }
}

fn handle_budget(db_path: Option<PathBuf>) {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = match rusqlite::Connection::open(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open: {e}");
            std::process::exit(2);
        }
    };
    match claude_core::ipc::models::list_subscriptions(&conn) {
        Ok(subs) => {
            println!(
                "{:<20} {:<10} {:>10} {:>10} {:>10} {:>6} {:>10} {}",
                "SUBSCRIPTION",
                "PROVIDER",
                "BUDGET",
                "SPENT",
                "REMAINING",
                "DAYS",
                "PROJECTED",
                "STATUS"
            );
            for s in &subs {
                if let Ok(Some(st)) = claude_core::ipc::budget::get_budget_status(&conn, &s.name) {
                    let status = if st.usage_pct >= 95.0 {
                        "CRITICAL"
                    } else if st.usage_pct >= 85.0 {
                        "HIGH"
                    } else if st.usage_pct >= 70.0 {
                        "WARN"
                    } else {
                        "OK"
                    };
                    println!(
                        "{:<20} {:<10} {:>10.2} {:>10.2} {:>10.2} {:>6} {:>10.2} {}",
                        s.name,
                        s.provider,
                        st.budget_usd,
                        st.total_spent,
                        st.remaining_budget,
                        st.days_remaining,
                        st.projected_total,
                        status
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("budget: {e}");
            std::process::exit(2);
        }
    }
}

fn handle_route(
    task_description: String,
    dry_run: bool,
    parallel: bool,
    db_path: Option<PathBuf>,
) {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = match rusqlite::Connection::open(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open: {e}");
            std::process::exit(2);
        }
    };
    if parallel {
        match claude_core::ipc::router::plan_parallel_execution(&conn, &task_description, 3) {
            Ok(plan) => println!(
                "{}",
                serde_json::to_string_pretty(&plan).unwrap_or_default()
            ),
            Err(e) => {
                eprintln!("parallel route: {e}");
                std::process::exit(2);
            }
        }
    } else if dry_run {
        let analysis = claude_core::ipc::router::analyze_task(&task_description);
        println!(
            "Analysis: {}",
            serde_json::to_string_pretty(&analysis).unwrap_or_default()
        );
        if let Ok(chain) = claude_core::ipc::router::fallback_chain(&conn, "") {
            println!("\nFallback chain:");
            for f in &chain {
                println!(
                    "  #{}: {} {} @ {} (free={}, degraded={})",
                    f.priority, f.provider, f.model, f.host, f.is_free, f.degraded
                );
            }
        }
    } else {
        match claude_core::ipc::router::route_task(&conn, &task_description) {
            Ok(Some(d)) => {
                println!("Model:      {}", d.model);
                println!("Provider:   {}", d.provider);
                println!("Host:       {}", d.host);
                println!("Reason:     {}", d.reason);
                println!("Confidence: {:.0}%", d.confidence * 100.0);
                println!("Est. Cost:  ${:.4}", d.estimated_cost);
            }
            Ok(None) => println!("No suitable model found"),
            Err(e) => {
                eprintln!("route: {e}");
                std::process::exit(2);
            }
        }
    }
}

fn handle_skills(agent: Option<String>, db_path: Option<PathBuf>) {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = match rusqlite::Connection::open(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open: {e}");
            std::process::exit(2);
        }
    };
    if let Some(agent_name) = agent {
        match claude_core::ipc::skills::get_skills_for_agent(&conn, &agent_name) {
            Ok(skills) => {
                println!(
                    "{:<20} {:<15} {:<15} {:>10} {}",
                    "SKILL", "AGENT", "HOST", "CONFIDENCE", "LAST_USED"
                );
                for s in &skills {
                    println!(
                        "{:<20} {:<15} {:<15} {:>10.2} {}",
                        s.skill, s.agent, s.host, s.confidence, s.last_used
                    );
                }
            }
            Err(e) => {
                eprintln!("skills: {e}");
                std::process::exit(2);
            }
        }
    } else {
        match claude_core::ipc::skills::get_skill_pool(&conn) {
            Ok(pool) => {
                println!(
                    "{:<20} {:<15} {:<15} {:>10} {}",
                    "SKILL", "AGENT", "HOST", "CONFIDENCE", "LAST_USED"
                );
                for (_, agents) in &pool {
                    for s in agents {
                        println!(
                            "{:<20} {:<15} {:<15} {:>10.2} {}",
                            s.skill, s.agent, s.host, s.confidence, s.last_used
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("skills: {e}");
                std::process::exit(2);
            }
        }
    }
}

fn handle_request_skill(skill: String, payload: String, db_path: Option<PathBuf>) {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = match rusqlite::Connection::open(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open: {e}");
            std::process::exit(2);
        }
    };
    match claude_core::ipc::skills::create_skill_request(&conn, &skill, &payload) {
        Ok(id) => {
            println!("Request created: {id}");
            if let Ok(Some((agent, host))) =
                claude_core::ipc::skills::find_best_agent(&conn, &skill)
            {
                let _ = claude_core::ipc::skills::assign_request(&conn, &id, &agent, &host);
                println!("Assigned to: {agent}@{host}");
            }
        }
        Err(e) => {
            eprintln!("request-skill: {e}");
            std::process::exit(2);
        }
    }
}

fn handle_respond_skill(request_id: String, result: String, db_path: Option<PathBuf>) {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = match rusqlite::Connection::open(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open: {e}");
            std::process::exit(2);
        }
    };
    match claude_core::ipc::skills::complete_skill_request(&conn, &request_id, &result) {
        Ok(()) => println!("Request {request_id} completed"),
        Err(e) => {
            eprintln!("respond-skill: {e}");
            std::process::exit(2);
        }
    }
}

fn handle_rate_skill(request_id: String, rating: f64, db_path: Option<PathBuf>) {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = match rusqlite::Connection::open(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open: {e}");
            std::process::exit(2);
        }
    };
    match claude_core::ipc::skills::rate_skill_response(&conn, &request_id, rating) {
        Ok(()) => println!("Rated request {request_id}: {rating}"),
        Err(e) => {
            eprintln!("rate-skill: {e}");
            std::process::exit(2);
        }
    }
}
