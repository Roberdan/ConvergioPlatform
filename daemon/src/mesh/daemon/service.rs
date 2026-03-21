// Daemon service: run_service, validate_config, WS handler, peer loop

use super::events::{now_ts, publish_event, relay_agent_activity_changes, relay_ipc_changes};
use super::net_utils::{
    collect_system_stats, is_ws_brain_request, load_peer_addrs, resolve_local_node_name,
    websocket_key,
};
use super::types::{DaemonConfig, DaemonState, InboundConnectionRateLimiter, MeshEvent};
use crate::mesh::net::apply_socket_tuning;
use crate::mesh::ws::{text_frame, websocket_accept};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};

pub async fn run_service(config: DaemonConfig) -> Result<(), String> {
    if !config.local_only {
        validate_config(&config)?;
    }

    // Ensure ALL tables are CRR-enabled at daemon startup
    {
        let conn = crate::mesh::sync::open_persistent_sync_conn(
            &config.db_path,
            config.crsqlite_path.as_deref(),
        )?;
        crate::mesh::sync::ensure_sync_schema_pub(&conn).map_err(|e| e.to_string())?;
    }

    // Ensure IPC schema exists
    {
        let conn = rusqlite::Connection::open(&config.db_path)
            .map_err(|e| format!("open db for IPC schema: {e}"))?;
        crate::ipc::ensure_ipc_schema(&conn).map_err(|e| e.to_string())?;
    }

    // Spawn IPC socket server
    let ipc_engine = std::sync::Arc::new(crate::ipc::IpcEngine::new(config.db_path.clone()));
    let ipc_socket = config
        .db_path
        .parent()
        .unwrap_or(std::path::Path::new("/tmp"))
        .join("ipc.sock");
    let ipc_eng = ipc_engine.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::ipc::socket::start_ipc_server(ipc_eng, ipc_socket).await {
            tracing::error!("IPC server failed: {e}");
        }
    });

    if config.local_only {
        // Local-only mode: only IPC socket + heartbeat, no mesh
        tracing::info!("daemon running in local-only mode (IPC socket only)");
        let hb_engine = ipc_engine.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(10));
            loop {
                ticker.tick().await;
                if let Err(e) = hb_engine.heartbeat_local_agents() {
                    tracing::warn!("heartbeat error: {e}");
                }
            }
        });
        // Wait for shutdown signal
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("daemon shutting down");
        return Ok(());
    }

    let bind_addr = format!("{}:{}", config.bind_ip, config.port);
    let listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| format!("mesh listen failed on {bind_addr}: {e}"))?;
    let inbound_rate_limiter = Arc::new(InboundConnectionRateLimiter::new(10, 100));
    let (tx, _) = broadcast::channel(256);
    let state = DaemonState {
        node_id: bind_addr.clone(),
        tx,
        heartbeats: Arc::new(RwLock::new(HashMap::new())),
    };

    for peer in load_peer_addrs(&config, &bind_addr) {
        tokio::spawn(connect_peer_loop(peer, state.clone(), config.clone()));
    }

    // Prune stale heartbeats every 60s (remove entries older than 5 minutes)
    let hb_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            let mut hb = hb_state.heartbeats.write().await;
            let now = now_ts();
            hb.retain(|_, ts| now.saturating_sub(*ts) < 300);
        }
    });

    // T2-00: HTTP API server on port+1 (e.g. 9421)
    let mesh_metrics = Arc::new(crate::mesh::observability::MeshMetrics::new());
    let log_buffer = Arc::new(crate::mesh::observability::LogBuffer::new(1000));
    let http_state = Arc::new(super::super::http_api::HttpState {
        daemon: state.clone(),
        db_path: config.db_path.clone(),
        crsqlite_path: config.crsqlite_path.clone(),
        start_time: std::time::Instant::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        metrics: mesh_metrics,
        logs: log_buffer,
    });
    let http_addr = format!("{}:{}", config.bind_ip, config.port + 1);
    let http_router = super::super::http_api::api_router().with_state(http_state);
    match tokio::net::TcpListener::bind(&http_addr).await {
        Ok(listener) => {
            tokio::spawn(async move {
                axum::serve(listener, http_router).await.ok();
            });
        }
        Err(e) => {
            eprintln!("WARNING: HTTP API bind failed on {http_addr}: {e}");
            eprintln!("Continuing without HTTP metrics API");
        }
    }

    // T2-03: Graceful shutdown handler
    let shutdown_state = state.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        publish_event(
            &shutdown_state,
            "shutdown",
            &shutdown_state.node_id,
            serde_json::json!({}),
        );
        // Give broadcast subscribers time to receive shutdown event
        tokio::time::sleep(Duration::from_millis(500)).await;
        std::process::exit(0);
    });

    // Local self-heartbeat: write own node to peer_heartbeats with system stats
    let local_config = config.clone();
    let local_node = resolve_local_node_name(&config.peers_conf_path, &config.bind_ip);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(10));
        loop {
            ticker.tick().await;
            let load = collect_system_stats();
            if let Ok(conn) = crate::mesh::sync::open_persistent_sync_conn(
                &local_config.db_path,
                local_config.crsqlite_path.as_deref(),
            ) {
                let load_json = serde_json::to_string(&load).unwrap_or_default();
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO peer_heartbeats (peer_name, last_seen, load_json) VALUES (?1, ?2, ?3)",
                    rusqlite::params![local_node, now_ts(), load_json],
                );
            }
        }
    });

    loop {
        let (mut stream, remote) = listener
            .accept()
            .await
            .map_err(|e| format!("mesh accept failed: {e}"))?;
        if let Err(err) = inbound_rate_limiter.check(remote) {
            tracing::warn!("inbound connection rejected from {remote}: {err}");
            let _ = stream.shutdown().await;
            continue;
        }
        let _ = apply_socket_tuning(&stream);
        let cfg = config.clone();
        let st = state.clone();
        let limiter = inbound_rate_limiter.clone();
        tokio::spawn(async move {
            let conn_id = format!("inbound-{remote}");
            let _ = super::daemon_sync::handle_socket(stream, conn_id, st, cfg, false).await;
            limiter.release(remote);
        });
    }
}

/// T1-07: Validate daemon config — fail fast with clear errors
fn validate_config(config: &DaemonConfig) -> Result<(), String> {
    // bind_ip must be a Tailscale IP (100.x.x.x) or localhost for security
    if !config.bind_ip.starts_with("100.")
        && config.bind_ip != "127.0.0.1"
        && config.bind_ip != "::1"
    {
        return Err(format!(
            "SECURITY: bind_ip '{}' is not a Tailscale IP (100.x.x.x) or localhost. \
             Binding to 0.0.0.0 would expose the mesh daemon to untrusted networks.",
            config.bind_ip
        ));
    }
    // DB path must exist
    if !config.db_path.exists() {
        return Err(format!("DB path does not exist: {:?}", config.db_path));
    }
    // crsqlite extension must exist if specified
    if let Some(ref ext) = config.crsqlite_path {
        let ext_path = std::path::Path::new(ext);
        // Check with platform extensions (.dylib, .so)
        let exists = ext_path.exists()
            || ext_path.with_extension("dylib").exists()
            || ext_path.with_extension("so").exists();
        if !exists {
            return Err(format!("crsqlite extension not found: {ext}"));
        }
    }
    // peers.conf must exist and be readable
    if !config.peers_conf_path.exists() {
        return Err(format!(
            "peers.conf not found: {:?}",
            config.peers_conf_path
        ));
    }
    if crate::mesh::auth::load_shared_secret(&config.peers_conf_path).is_none() {
        return Err(format!(
            "mesh auth requires non-empty [mesh].shared_secret in peers.conf: {:?}",
            config.peers_conf_path
        ));
    }
    Ok(())
}

pub async fn handle_ws_client(
    mut stream: TcpStream,
    request: &str,
    state: DaemonState,
) -> Result<(), String> {
    let key = websocket_key(request).ok_or_else(|| "missing websocket key".to_string())?;
    let accept = websocket_accept(&key);
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let mut sub = state.tx.subscribe();
    let snapshot = {
        let heartbeats = state.heartbeats.read().await;
        json!({"kind":"heartbeat_snapshot","node":state.node_id,"ts":now_ts(),"payload":{"nodes":*heartbeats}})
    };
    stream
        .write_all(&text_frame(&snapshot.to_string()))
        .await
        .map_err(|e| e.to_string())?;
    while let Ok(event) = sub.recv().await {
        let payload = serde_json::to_string(&event).map_err(|e| e.to_string())?;
        if stream.write_all(&text_frame(&payload)).await.is_err() {
            break;
        }
    }
    Ok(())
}

async fn connect_peer_loop(peer: String, state: DaemonState, config: DaemonConfig) {
    let mut backoff_secs = 3u64;
    loop {
        match TcpStream::connect(&peer).await {
            Ok(stream) => {
                backoff_secs = 3; // reset on success
                let _ = apply_socket_tuning(&stream);
                let _ = super::daemon_sync::handle_socket(
                    stream,
                    format!("peer-{peer}"),
                    state.clone(),
                    config.clone(),
                    true,
                )
                .await;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(60); // exponential backoff, max 60s
            }
        }
    }
}
