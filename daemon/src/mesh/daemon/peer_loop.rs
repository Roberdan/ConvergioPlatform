// Config validation and outbound peer connection loop.
// Separated from service.rs to keep each file under 250 lines.

use super::types::{DaemonConfig, DaemonState};
use crate::mesh::net::apply_socket_tuning;
use std::time::Duration;
use tokio::net::TcpStream;

/// T1-07: Validate daemon config — fail fast with clear errors
pub fn validate_config(config: &DaemonConfig) -> Result<(), String> {
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

pub(super) async fn connect_peer_loop(peer: String, state: DaemonState, config: DaemonConfig) {
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
