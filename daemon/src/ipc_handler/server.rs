use std::path::PathBuf;
use tracing::{info, warn};

use super::types::IpcHandlerError;
use super::utils::{default_db_path, default_peers_conf};

pub async fn run_serve(
    bind: String,
    static_dir: Option<PathBuf>,
    crsqlite_path: Option<String>,
    dev_mode: bool,
    mesh_enabled: bool,
) -> Result<(), IpcHandlerError> {
    // Initialise dev-mode flag before any request is handled.
    claude_core::server::middleware::set_dev_mode(dev_mode);

    // In dev-mode with no auth token, force localhost-only binding.
    let effective_bind = claude_core::server::resolve_bind_addr(&bind, dev_mode);

    let dir = static_dir.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        claude_core::server::resolve_dashboard_static_dir(PathBuf::from(home).join(".claude"))
    });
    info!("claude-core serve → {effective_bind} (static: {dir:?})");
    eprintln!("claude-core serve → {effective_bind} (static: {dir:?})");

    // Unified daemon: mesh CRDT sync + background sync loop alongside HTTP server
    if mesh_enabled {
        let sync_db = default_db_path();
        if let Ok(sync_conn) = rusqlite::Connection::open(&sync_db) {
            let sync_conn = std::sync::Arc::new(std::sync::Mutex::new(sync_conn));
            let interval = claude_core::background_sync::resolve_interval_secs(None);
            claude_core::background_sync::spawn_sync_loop(sync_conn, interval);
        }

        let db_path = default_db_path();
        let peers_conf = default_peers_conf();
        let crsqlite_clone = crsqlite_path.clone();
        tokio::spawn(async move {
            let config = claude_core::mesh::daemon::DaemonConfig {
                bind_ip: claude_core::mesh::daemon::detect_tailscale_ip()
                    .unwrap_or_else(|| "127.0.0.1".to_string()),
                port: 9420,
                peers_conf_path: peers_conf,
                db_path,
                crsqlite_path: crsqlite_clone,
                local_only: false,
            };
            info!(
                "mesh service starting on {}:{}",
                config.bind_ip, config.port
            );
            eprintln!("mesh service → {}:{}", config.bind_ip, config.port);
            if let Err(err) = claude_core::mesh::daemon::run_service(config).await {
                warn!("mesh service failed: {err}");
                eprintln!("mesh service failed (non-fatal): {err}");
            }
        });
    }

    if let Err(err) = claude_core::server::run(&effective_bind, dir, crsqlite_path).await {
        warn!("server failed: {err}");
        return Err(IpcHandlerError::ServerFailed(format!(
            "server failed: {err}"
        )));
    }
    Ok(())
}

pub async fn run_daemon(
    bind_ip: Option<String>,
    port: u16,
    peers_conf: Option<PathBuf>,
    db_path: Option<PathBuf>,
    crsqlite_path: Option<String>,
    local_only: bool,
) -> Result<(), IpcHandlerError> {
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
        return Err(IpcHandlerError::ServerFailed(format!(
            "daemon start failed: {err}"
        )));
    }
    Ok(())
}
