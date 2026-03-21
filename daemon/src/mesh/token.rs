// Token signing and verification (HMAC-SHA256)
// Format: base64url(payload_json) + "." + base64url(hmac_signature)

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenPayload {
    pub role: String,
    pub capabilities: Vec<String>,
    pub coordinator_ip: String,
    pub nonce: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum TokenError {
    #[error("invalid signature")]
    InvalidSignature,
    #[error("token expired")]
    Expired,
    #[error("token already used")]
    AlreadyUsed,
    #[error("malformed token")]
    MalformedToken,
    #[error("database error: {0}")]
    DatabaseError(String),
}

/// Initialise the used_tokens table. Safe to call multiple times.
pub fn init_token_db(db: &Connection) -> Result<(), TokenError> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS used_tokens (
            nonce   TEXT PRIMARY KEY,
            used_at TEXT NOT NULL
        );",
    )
    .map_err(|e| TokenError::DatabaseError(e.to_string()))
}

/// Generate a single-use HMAC-signed invite token.
///
/// Returns a string of the form `<base64url_payload>.<base64url_signature>`.
pub fn generate_token(
    secret: &[u8],
    role: &str,
    capabilities: Vec<String>,
    coordinator_ip: &str,
    ttl_minutes: i64,
) -> Result<String, TokenError> {
    let payload = TokenPayload {
        role: role.to_string(),
        capabilities,
        coordinator_ip: coordinator_ip.to_string(),
        nonce: Uuid::new_v4().to_string(),
        expires_at: Utc::now() + Duration::minutes(ttl_minutes),
    };

    let payload_json =
        serde_json::to_string(&payload).map_err(|e| TokenError::DatabaseError(e.to_string()))?;

    let payload_b64 = URL_SAFE_NO_PAD.encode(payload_json.as_bytes());

    let mut mac =
        HmacSha256::new_from_slice(secret).map_err(|e| TokenError::DatabaseError(e.to_string()))?;
    mac.update(payload_b64.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_b64 = URL_SAFE_NO_PAD.encode(signature);

    Ok(format!("{}.{}", payload_b64, signature_b64))
}

/// Validate a token string.
///
/// Checks (in order):
/// 1. Structure is well-formed
/// 2. HMAC signature is valid
/// 3. Token has not expired
/// 4. Nonce has not been used before
///
/// On success the nonce is marked as used — the token cannot be reused.
pub fn validate_token(
    token_str: &str,
    secret: &[u8],
    db: &Connection,
) -> Result<TokenPayload, TokenError> {
    let mut parts = token_str.splitn(2, '.');
    let payload_b64 = parts.next().ok_or(TokenError::MalformedToken)?;
    let signature_b64 = parts.next().ok_or(TokenError::MalformedToken)?;

    // Verify signature before decoding payload (prevents oracle attacks)
    let expected_sig = URL_SAFE_NO_PAD
        .decode(signature_b64)
        .map_err(|_| TokenError::MalformedToken)?;

    let mut mac = HmacSha256::new_from_slice(secret).map_err(|_| TokenError::MalformedToken)?;
    mac.update(payload_b64.as_bytes());
    mac.verify_slice(&expected_sig)
        .map_err(|_| TokenError::InvalidSignature)?;

    // Decode and deserialise payload
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| TokenError::MalformedToken)?;
    let payload: TokenPayload =
        serde_json::from_slice(&payload_bytes).map_err(|_| TokenError::MalformedToken)?;

    // Check expiry
    if Utc::now() > payload.expires_at {
        return Err(TokenError::Expired);
    }

    // Check nonce not already consumed
    let count: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM used_tokens WHERE nonce = ?1",
            params![payload.nonce],
            |row| row.get(0),
        )
        .map_err(|e| TokenError::DatabaseError(e.to_string()))?;

    if count > 0 {
        return Err(TokenError::AlreadyUsed);
    }

    // Mark nonce as used (single-use enforcement)
    db.execute(
        "INSERT INTO used_tokens (nonce, used_at) VALUES (?1, ?2)",
        params![payload.nonce, Utc::now().to_rfc3339()],
    )
    .map_err(|e| TokenError::DatabaseError(e.to_string()))?;

    Ok(payload)
}

/// Revoke a token by nonce, preventing future use even if the token has not
/// been consumed yet.
pub fn revoke_token(nonce: &str, db: &Connection) -> Result<(), TokenError> {
    db.execute(
        "INSERT OR IGNORE INTO used_tokens (nonce, used_at) VALUES (?1, ?2)",
        params![nonce, Utc::now().to_rfc3339()],
    )
    .map_err(|e| TokenError::DatabaseError(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
#[path = "token_tests.rs"]
mod tests;
