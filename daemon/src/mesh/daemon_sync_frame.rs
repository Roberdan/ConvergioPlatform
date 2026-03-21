use crate::mesh::sync::{self, MeshSyncFrame};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};

use crate::mesh::daemon::{
    is_ws_brain_request, publish_event, relay_agent_activity_changes, relay_ipc_changes,
    DaemonConfig, DaemonState,
};

pub(super) async fn process_frame(
    frame: &MeshSyncFrame,
    state: &DaemonState,
    config: &DaemonConfig,
    out_tx: &mpsc::Sender<MeshSyncFrame>,
    sync_peer: &Arc<RwLock<String>>,
) -> Result<(), String> {
    match frame {
        MeshSyncFrame::Heartbeat { node, ts } => {
            *sync_peer.write().await = node.clone();
            state.heartbeats.write().await.insert(node.clone(), *ts);
            // T1-06: Persist heartbeat to DB with crsqlite loaded (CRR triggers need it)
            if let Ok(conn) =
                sync::open_persistent_sync_conn(&config.db_path, config.crsqlite_path.as_deref())
            {
                let peer_name = resolve_peer_name(&config.peers_conf_path, node);
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO peer_heartbeats (peer_name, last_seen) \
                     VALUES (?1, ?2) \
                     ON CONFLICT(peer_name) DO UPDATE SET last_seen = excluded.last_seen",
                    rusqlite::params![peer_name, ts],
                );
            }
            publish_event(state, "heartbeat", node, json!({ "ts": ts }));
        }
        MeshSyncFrame::Delta {
            node,
            sent_at_ms,
            changes,
            ..
        } => {
            *sync_peer.write().await = node.clone();
            let summary = sync::apply_delta_frame(
                &config.db_path,
                config.crsqlite_path.as_deref(),
                node,
                *sent_at_ms,
                changes,
            )?;
            let _ = out_tx
                .send(MeshSyncFrame::Ack {
                    node: state.node_id.clone(),
                    applied: summary.applied,
                    latency_ms: summary.latency_ms,
                    last_db_version: summary.last_db_version,
                })
                .await;
            relay_agent_activity_changes(state, node, changes);
            relay_ipc_changes(state, node, changes);
            publish_event(
                state,
                "sync_delta",
                node,
                json!({"received": changes.len(), "applied": summary.applied,
                       "latency_ms": summary.latency_ms}),
            );
        }
        MeshSyncFrame::Ack {
            node,
            applied,
            latency_ms,
            last_db_version,
        } => {
            *sync_peer.write().await = node.clone();
            publish_event(
                state,
                "sync_ack",
                node,
                json!({"applied": applied, "latency_ms": latency_ms,
                       "last_db_version": last_db_version}),
            );
        }
        // Auth frames are handled in handshake, not in main loop
        MeshSyncFrame::AuthChallenge { .. }
        | MeshSyncFrame::AuthResponse { .. }
        | MeshSyncFrame::AuthResult { .. } => {}
    }
    Ok(())
}

pub(super) fn resolve_peer_name(peers_conf_path: &std::path::Path, node: &str) -> String {
    let ip = node.split(':').next().unwrap_or(node);
    if let Ok(content) = std::fs::read_to_string(peers_conf_path) {
        let mut section_name: Option<String> = None;
        for line in content.lines().map(str::trim) {
            if line.starts_with('[') && line.ends_with(']') {
                section_name = Some(line[1..line.len() - 1].to_string());
            } else if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "tailscale_ip" && value.trim() == ip {
                    if let Some(name) = &section_name {
                        return name.clone();
                    }
                }
            }
        }
    }
    node.to_string()
}

pub(super) async fn maybe_ws_request_head(stream: &mut TcpStream) -> Option<String> {
    let mut probe = [0_u8; 2048];
    let peeked = tokio::time::timeout(Duration::from_millis(150), stream.peek(&mut probe))
        .await
        .ok()?
        .ok()?;
    if peeked == 0 {
        return None;
    }
    let head = String::from_utf8_lossy(&probe[..peeked]).to_string();
    if !is_ws_brain_request(&head) {
        return None;
    }
    let read = stream.read(&mut probe).await.ok()?;
    Some(String::from_utf8_lossy(&probe[..read]).to_string())
}
