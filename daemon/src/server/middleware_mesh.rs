//! Mesh HMAC authentication for peer-to-peer sync requests.
//!
//! Verifies X-Mesh-Timestamp + X-Mesh-Signature headers using the
//! shared_secret from peers.conf. Extracted from middleware.rs.

use crate::mesh::auth::load_shared_secret;
use crate::security::jwt::AgentClaims;
use std::sync::OnceLock;

/// Cached shared secret from peers.conf — loaded once at first use.
static MESH_SECRET: OnceLock<Option<Vec<u8>>> = OnceLock::new();

fn get_mesh_secret() -> &'static Option<Vec<u8>> {
    MESH_SECRET.get_or_init(|| {
        let conf_path = std::path::PathBuf::from(
            crate::background_sync_http::peers_conf_path_from_env(),
        );
        load_shared_secret(&conf_path)
    })
}

/// Verify mesh HMAC signature from a peer sync request.
/// Signed message = "{timestamp}:{METHOD}:{path_and_query}".
pub(crate) fn verify_mesh_hmac(
    timestamp: &str,
    signature: &str,
    path_and_query: &str,
    method: &str,
) -> Result<Option<AgentClaims>, ()> {
    // Reject stale timestamps (>5 min drift)
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
    let message = format!("{timestamp}:{method}:{path_and_query}");

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
