//! Mesh HMAC authentication for peer-to-peer sync requests.
//!
//! Verifies X-Mesh-Timestamp + X-Mesh-Signature headers using the
//! shared_secret from peers.conf. Extracted from middleware.rs.

use crate::mesh::auth::load_shared_secret;
use crate::security::jwt::AgentClaims;

/// Verify mesh HMAC signature from a peer sync request.
/// Header format: X-Mesh-Timestamp + X-Mesh-Signature (hex-encoded HMAC).
/// Message = "{timestamp}:{method}:{path}", method inferred as GET or POST.
pub(crate) fn verify_mesh_hmac(
    timestamp: &str,
    signature: &str,
    path: &str,
) -> Result<Option<AgentClaims>, ()> {
    // Reject stale timestamps (>5 min drift)
    let ts: i64 = timestamp.parse().map_err(|_| ())?;
    let now = chrono::Utc::now().timestamp();
    if (now - ts).unsigned_abs() > 300 {
        tracing::warn!("mesh HMAC rejected: timestamp drift {}s", now - ts);
        return Err(());
    }

    let conf_path = std::path::PathBuf::from(
        crate::background_sync_http::peers_conf_path_from_env(),
    );
    let secret = load_shared_secret(&conf_path).ok_or_else(|| {
        tracing::warn!("mesh HMAC rejected: no shared_secret in peers.conf");
    })?;

    let sig_bytes = hex::decode(signature).map_err(|_| ())?;

    // Try both GET and POST since we don't have the method in headers
    for method in &["GET", "POST"] {
        let message = format!("{timestamp}:{method}:{path}");
        if let Ok(true) = crate::mesh::auth::verify_hmac(
            &secret,
            message.as_bytes(),
            &sig_bytes,
        ) {
            tracing::debug!(path, "mesh HMAC auth OK via {method}");
            return Ok(None);
        }
    }
    tracing::warn!(path, "mesh HMAC rejected: signature mismatch");
    Err(())
}
