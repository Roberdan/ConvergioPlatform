use std::path::PathBuf;
use tracing::{info, warn};

use super::utils::{default_db_path, default_peers_conf};

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
