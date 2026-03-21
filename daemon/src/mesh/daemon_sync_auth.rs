use crate::mesh::auth;
use crate::mesh::sync::{self, MeshSyncFrame};
use serde_json::json;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

use super::daemon::{publish_event, DaemonState};

/// T1-09: Run challenge-response auth handshake for an outbound connection.
/// Outbound: wait for challenge from remote, respond with HMAC.
pub(super) async fn run_outbound_auth(
    read_half: &mut OwnedReadHalf,
    write_half: &mut OwnedWriteHalf,
    peer_quota: &mut sync::PeerQuota,
    state: &DaemonState,
    secret: &[u8],
) -> Result<(), String> {
    match sync::read_frame_with_quota(read_half, peer_quota).await? {
        Some(framed) => {
            let MeshSyncFrame::AuthChallenge { nonce, .. } = framed.frame else {
                peer_quota.release(framed.payload_len as usize);
                return Err("expected AuthChallenge".into());
            };
            let hmac = auth::compute_hmac(secret, &nonce)?;
            sync::write_frame(
                write_half,
                &MeshSyncFrame::AuthResponse {
                    hmac,
                    node: state.node_id.clone(),
                },
            )
            .await?;
            peer_quota.release(framed.payload_len as usize);
            match sync::read_frame_with_quota(read_half, peer_quota).await? {
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
    Ok(())
}

/// T1-09: Run challenge-response auth handshake for an inbound connection.
/// Inbound: send challenge, verify HMAC response.
pub(super) async fn run_inbound_auth(
    read_half: &mut OwnedReadHalf,
    write_half: &mut OwnedWriteHalf,
    peer_quota: &mut sync::PeerQuota,
    state: &DaemonState,
    secret: &[u8],
) -> Result<(), String> {
    let nonce = auth::generate_nonce();
    sync::write_frame(
        write_half,
        &MeshSyncFrame::AuthChallenge {
            nonce: nonce.clone(),
            node: state.node_id.clone(),
        },
    )
    .await?;
    match sync::read_frame_with_quota(read_half, peer_quota).await? {
        Some(framed) => {
            let result = match framed.frame {
                MeshSyncFrame::AuthResponse { hmac, node } => {
                    if auth::verify_hmac(secret, &nonce, &hmac)? {
                        sync::write_frame(
                            write_half,
                            &MeshSyncFrame::AuthResult {
                                ok: true,
                                reason: String::new(),
                            },
                        )
                        .await?;
                        publish_event(state, "auth_ok", &node, json!({}));
                        Ok(())
                    } else {
                        sync::write_frame(
                            write_half,
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
    Ok(())
}
