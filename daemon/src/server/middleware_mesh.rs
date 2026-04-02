//! Mesh HMAC authentication for peer-to-peer sync requests.
//!
//! Verifies X-Mesh-Timestamp + X-Mesh-Signature + optional X-Mesh-Body-Hash
//! headers using the shared_secret from peers.conf. Extracted from middleware.rs.

use crate::mesh::auth::load_shared_secret;
use crate::mesh::peers::peers_conf_path_from_env;
use crate::security::jwt::AgentClaims;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

/// Cached shared secret from peers.conf — loaded once at first use.
static MESH_SECRET: OnceLock<Option<Vec<u8>>> = OnceLock::new();

fn get_mesh_secret() -> &'static Option<Vec<u8>> {
    MESH_SECRET.get_or_init(|| {
        let conf_path = std::path::PathBuf::from(peers_conf_path_from_env());
        load_shared_secret(&conf_path)
    })
}

/// Verify body bytes match the claimed SHA-256 hash (constant-time comparison).
pub(crate) fn verify_body_hash(body: &[u8], claimed_hash: &str) -> bool {
    let computed = hex::encode(Sha256::digest(body));
    constant_time_eq::constant_time_eq(computed.as_bytes(), claimed_hash.as_bytes())
}

/// Verify mesh HMAC signature from a peer sync request.
/// Signed message = "{timestamp}:{METHOD}:{path_and_query}" (legacy) or
/// "{timestamp}:{METHOD}:{path_and_query}:{body_hash}" (with body coverage).
pub(crate) fn verify_mesh_hmac(
    timestamp: &str,
    signature: &str,
    path_and_query: &str,
    method: &str,
    body_hash: Option<&str>,
) -> Result<Option<AgentClaims>, ()> {
    let ts: i64 = timestamp.parse().map_err(|_| ())?;
    let now = chrono::Utc::now().timestamp();
    if (now - ts).unsigned_abs() > 300 {
        tracing::warn!("mesh HMAC rejected: timestamp drift {}s", now - ts);
        return Err(());
    }

    let secret = get_mesh_secret().as_ref().ok_or_else(|| {
        tracing::warn!("mesh HMAC rejected: no shared_secret in peers.conf");
    })?;

    let sig_bytes = hex::decode(signature).map_err(|_| ())?;
    let message = match body_hash {
        Some(bh) => format!("{timestamp}:{method}:{path_and_query}:{bh}"),
        None => format!("{timestamp}:{method}:{path_and_query}"),
    };

    match crate::mesh::auth::verify_hmac(secret, message.as_bytes(), &sig_bytes) {
        Ok(true) => {
            tracing::debug!(path_and_query, method, "mesh HMAC auth OK");
            Ok(None)
        }
        _ => {
            tracing::warn!(path_and_query, method, "mesh HMAC rejected: signature mismatch");
            Err(())
        }
    }
}
