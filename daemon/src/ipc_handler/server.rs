use std::path::PathBuf;
use tracing::{info, warn};

use super::types::IpcHandlerError;
use super::utils::{default_db_path, default_peers_conf};

/// Kill any existing process listening on a port (prevents "Address already in use").
fn kill_stale_listeners(port: u16) {
    let output = std::process::Command::new("lsof")
        .args(["-ti", &format!(":{port}")])
        .output();
    if let Ok(out) = output {
        let pids = String::from_utf8_lossy(&out.stdout);
        let my_pid = std::process::id();
        for pid_str in pids.split_whitespace() {
            if let Ok(pid) = pid_str.parse::<u32>() {
                if pid != my_pid {
                    info!("killing stale process {pid} on port {port}");
                    if let Err(e) = std::process::Command::new("kill").args(["-9", pid_str]).output() {
                        warn!("kill_stale_listeners: kill failed for pid {pid}: {e}");
                    }
                }
            }
        }
        if !pids.is_empty() {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}

pub async fn run_serve(
    bind: String,
    static_dir: Option<PathBuf>,
    crsqlite_path: Option<String>,
    dev_mode: bool,
    mesh_enabled: bool,
) -> Result<(), IpcHandlerError> {
    // Initialise dev-mode flag before any request is handled.
    convergio_core::server::middleware::set_dev_mode(dev_mode);

    // In dev-mode with no auth token, force localhost-only binding.
    let effective_bind = convergio_core::server::resolve_bind_addr(&bind, dev_mode);

    let dir = static_dir.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        convergio_core::server::resolve_dashboard_static_dir(PathBuf::from(home).join(".claude"))
    });
    info!("claude-core serve → {effective_bind} (static: {dir:?})");
    eprintln!("claude-core serve → {effective_bind} (static: {dir:?})");

    // Kill stale processes on our ports before binding
    kill_stale_listeners(8420);
    if mesh_enabled {
        kill_stale_listeners(9420);
    }

    // Unified daemon: ONE shared IPC engine for HTTP + mesh + Ali
    let db_path = default_db_path();
    let shared_ipc = std::sync::Arc::new(convergio_core::ipc::IpcEngine::new(db_path.clone()));
    if let Ok(conn) = shared_ipc.open_conn() {
        if let Err(e) = convergio_core::ipc::ensure_ipc_schema(&conn) {
            warn!("ensure_ipc_schema failed: {e}");
        }
    }

    // ServerState uses the shared IPC engine
    let server_state = convergio_core::server::state::ServerState::with_ipc_engine(
        db_path.clone(),
        crsqlite_path.clone(),
        shared_ipc.clone(),
    );

    let readiness_checks = convergio_core::server::api_node_readiness::run_checks();
    let readiness_summary = convergio_core::server::api_node_readiness::summarize_for_boot(
        &readiness_checks,
    );
    if !readiness_summary.warning_failures.is_empty() {
        warn!(
            warnings = ?readiness_summary.warning_failures,
            "boot readiness completed with warnings"
        );
    }
    if !readiness_summary.blocking_failures.is_empty() {
        let detail = readiness_summary.blocking_failures.join("; ");
        warn!(blocking = ?readiness_summary.blocking_failures, "boot readiness failed");
        return Err(IpcHandlerError::ServerFailed(format!(
            "boot readiness failed: {detail}"
        )));
    }
    info!("boot readiness passed");

    if mesh_enabled {
        // DEPRECATED v20: HTTP LWW sync disabled. CRDT over TCP (port 9420)
        // is the sole replication path. Code kept for potential fallback.
        // if let Ok(sync_conn) = rusqlite::Connection::open(&db_path) {
        //     let sync_conn = std::sync::Arc::new(std::sync::Mutex::new(sync_conn));
        //     let interval = convergio_core::background_sync::resolve_interval_secs(None);
        //     convergio_core::background_sync::spawn_sync_loop(sync_conn, interval);
        // }

        // Mesh daemon (shares same IPC engine via DB path — Ali spawns inside)
        let peers_conf = default_peers_conf();
        let crsqlite_clone = crsqlite_path.clone();
        let mesh_db = db_path.clone();
        tokio::spawn(async move {
            let config = convergio_core::mesh::daemon::DaemonConfig {
                bind_ip: convergio_core::mesh::daemon::detect_tailscale_ip()
                    .unwrap_or_else(|| "127.0.0.1".to_string()),
                port: 9420,
                peers_conf_path: peers_conf,
                db_path: mesh_db,
                crsqlite_path: crsqlite_clone,
                local_only: false,
            };
            info!("mesh service starting on {}:{}", config.bind_ip, config.port);
            eprintln!("mesh service → {}:{}", config.bind_ip, config.port);
            if let Err(err) = convergio_core::mesh::daemon::run_service(config).await {
                warn!("mesh service failed: {err}");
                eprintln!("mesh service failed (non-fatal): {err}");
            }
        });

        // Spawn Ali on the SHARED IPC engine (same Notify as ServerState)
        convergio_core::orchestrator::spawn_ali(shared_ipc.clone(), db_path);
    }

    if let Err(err) = convergio_core::server::run_with_state(&effective_bind, dir, server_state).await
    {
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
            .or_else(|| match std::env::var("TAILSCALE_IP") { Ok(v) => Some(v), Err(_) => None })
            .or_else(convergio_core::mesh::daemon::detect_tailscale_ip)
            .unwrap_or_else(|| "0.0.0.0".to_string())
    };
    let config = convergio_core::mesh::daemon::DaemonConfig {
        bind_ip: resolved_ip,
        port,
        peers_conf_path: peers_conf.unwrap_or_else(default_peers_conf),
        db_path: db_path.unwrap_or_else(default_db_path),
        crsqlite_path,
        local_only,
    };
    if let Err(err) = convergio_core::mesh::daemon::run_service(config).await {
        return Err(IpcHandlerError::ServerFailed(format!(
            "daemon start failed: {err}"
        )));
    }
    Ok(())
}
