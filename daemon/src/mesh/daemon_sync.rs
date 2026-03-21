#[path = "daemon_sync_loops.rs"]
mod loops;
#[path = "daemon_sync_frame.rs"]
mod frame;

use crate::mesh::auth;
use crate::mesh::sync::{self, MeshSyncFrame};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

use super::{
    handle_ws_client, now_ts, publish_event, DaemonConfig, DaemonState,
};

/// Handle a peer connection. `is_outbound` = true spawns the delta loop (only outbound sends changes).
/// Inbound connections only receive frames and send heartbeats/acks.
/// T1-09: Challenge-response auth required before any data exchange.
pub(super) async fn handle_socket(
    mut stream: tokio::net::TcpStream,
    conn_id: String,
    state: DaemonState,
    config: DaemonConfig,
    is_outbound: bool,
) -> Result<(), String> {
    if let Some(head) = frame::maybe_ws_request_head(&mut stream).await {
        return handle_ws_client(stream, &head, state).await;
    }
    let (mut read_half, mut write_half) = stream.into_split();
    let mut peer_quota = sync::PeerQuota::new();

    // T1-09: Peer authentication — shared secret from [mesh] section of peers.conf
    let secret = load_required_shared_secret(&config.peers_conf_path)?;
    if is_outbound {
        // Outbound: wait for challenge, respond with HMAC
        match sync::read_frame_with_quota(&mut read_half, &mut peer_quota).await? {
            Some(framed) => {
                let MeshSyncFrame::AuthChallenge { nonce, .. } = framed.frame else {
                    peer_quota.release(framed.payload_len as usize);
                    return Err("expected AuthChallenge".into());
                };
                let hmac = auth::compute_hmac(&secret, &nonce)?;
                sync::write_frame(
                    &mut write_half,
                    &MeshSyncFrame::AuthResponse {
                        hmac,
                        node: state.node_id.clone(),
                    },
                )
                .await?;
                peer_quota.release(framed.payload_len as usize);
                match sync::read_frame_with_quota(&mut read_half, &mut peer_quota).await? {
                    Some(reply) => {
                        let result = match reply.frame {
                            MeshSyncFrame::AuthResult { ok: true, .. } => Ok(()),
                            MeshSyncFrame::AuthResult {
                                ok: false, reason, ..
                            } => Err(format!("auth rejected: {reason}")),
                            _ => Err("unexpected frame during auth".into()),
                        };
                        peer_quota.release(reply.payload_len as usize);
                        result?;
                    }
                    None => return Err("unexpected EOF during auth".into()),
                }
            }
            _ => return Err("expected AuthChallenge".into()),
        }
    } else {
        // Inbound: send challenge, verify response
        let nonce = auth::generate_nonce();
        sync::write_frame(
            &mut write_half,
            &MeshSyncFrame::AuthChallenge {
                nonce: nonce.clone(),
                node: state.node_id.clone(),
            },
        )
        .await?;
        match sync::read_frame_with_quota(&mut read_half, &mut peer_quota).await? {
            Some(framed) => {
                let result = match framed.frame {
                    MeshSyncFrame::AuthResponse { hmac, node } => {
                        if auth::verify_hmac(&secret, &nonce, &hmac)? {
                            sync::write_frame(
                                &mut write_half,
                                &MeshSyncFrame::AuthResult {
                                    ok: true,
                                    reason: String::new(),
                                },
                            )
                            .await?;
                            publish_event(&state, "auth_ok", &node, json!({}));
                            Ok(())
                        } else {
                            sync::write_frame(
                                &mut write_half,
                                &MeshSyncFrame::AuthResult {
                                    ok: false,
                                    reason: "HMAC mismatch".into(),
                                },
                            )
                            .await?;
                            Err(format!("auth failed for {node}: HMAC mismatch"))
                        }
                    }
                    _ => Err("expected AuthResponse".into()),
                };
                peer_quota.release(framed.payload_len as usize);
                result?;
            }
            _ => return Err("expected AuthResponse".into()),
        }
    }
    // Auth passed

    let (out_tx, mut out_rx) = mpsc::channel::<MeshSyncFrame>(64);
    let writer = tokio::spawn(async move {
        while let Some(frame) = out_rx.recv().await {
            if sync::write_frame(&mut write_half, &frame).await.is_err() {
                break;
            }
        }
    });
    let _ = out_tx
        .send(MeshSyncFrame::Heartbeat {
            node: state.node_id.clone(),
            ts: now_ts(),
        })
        .await;
    let sync_peer = Arc::new(RwLock::new(conn_id.clone()));
    loops::spawn_heartbeat_loop(out_tx.clone(), state.node_id.clone());
    // Only outbound connections send deltas — prevents duplicate delta loops
    if is_outbound {
        loops::spawn_delta_loop(
            out_tx.clone(),
            sync_peer.clone(),
            state.node_id.clone(),
            config.clone(),
        );
    }
    let mut consecutive_errors: u32 = 0;
    loop {
        let framed = match sync::read_frame_with_quota(&mut read_half, &mut peer_quota).await? {
            Some(framed) => framed,
            None => break,
        };
        let frame = framed.frame;
        let payload_len = framed.payload_len;
        if let Err(err) = frame::process_frame(&frame, &state, &config, &out_tx, &sync_peer).await {
            peer_quota.release(payload_len as usize);
            consecutive_errors = consecutive_errors.saturating_add(1);
            let peer = sync_peer.read().await.clone();
            let _ = sync::record_sync_error(
                &config.db_path,
                config.crsqlite_path.as_deref(),
                &peer,
                &err,
            );
            publish_event(&state, "sync_error", &peer, json!({ "error": err }));
            if consecutive_errors > 3 {
                let delay = std::cmp::min(consecutive_errors as u64 * 2, 30);
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
        } else {
            peer_quota.release(payload_len as usize);
            consecutive_errors = 0;
        }
    }
    drop(out_tx);
    let _ = writer.await;
    Ok(())
}

pub(super) fn load_required_shared_secret(peers_conf: &std::path::Path) -> Result<Vec<u8>, String> {
    auth::load_shared_secret(peers_conf).ok_or_else(|| {
        format!(
            "mesh auth requires non-empty [mesh].shared_secret in peers.conf: {:?}",
            peers_conf
        )
    })
}
