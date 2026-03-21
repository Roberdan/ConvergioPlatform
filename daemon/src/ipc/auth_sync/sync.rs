use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::validate::{decrypt_token, derive_key, encrypt_token};

// --- T8047: DB operations ---

pub fn store_token(
    conn: &Connection,
    service: &str,
    plaintext: &str,
    shared_secret: &str,
) -> rusqlite::Result<()> {
    info!(service, "storing encrypted token");
    let key = derive_key(shared_secret);
    let (encrypted, nonce) = encrypt_token(&key, plaintext);
    let host = local_host();
    conn.execute(
        "INSERT OR REPLACE INTO ipc_auth_tokens (service, encrypted_token, nonce, host, updated_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        params![service, encrypted, nonce.to_vec(), host],
    )?;
    Ok(())
}

pub fn get_token(
    conn: &Connection,
    service: &str,
    shared_secret: &str,
) -> rusqlite::Result<Option<String>> {
    let host = local_host();
    let mut stmt = conn.prepare(
        "SELECT encrypted_token, nonce FROM ipc_auth_tokens WHERE service=?1 AND host=?2",
    )?;
    let result = stmt.query_row(params![service, host], |row| {
        let encrypted: Vec<u8> = row.get(0)?;
        let nonce_vec: Vec<u8> = row.get(1)?;
        Ok((encrypted, nonce_vec))
    });
    match result {
        Ok((encrypted, nonce_vec)) => {
            let nonce: [u8; 12] = nonce_vec
                .try_into()
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let key = derive_key(shared_secret);
            match decrypt_token(&key, &encrypted, &nonce) {
                Ok(plain) => Ok(Some(plain)),
                Err(_) => Ok(None),
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub service: String,
    pub host: String,
    pub updated_at: String,
}

pub fn list_tokens(conn: &Connection) -> rusqlite::Result<Vec<TokenInfo>> {
    let mut stmt =
        conn.prepare("SELECT service, host, updated_at FROM ipc_auth_tokens ORDER BY service")?;
    let rows = stmt.query_map([], |row| {
        Ok(TokenInfo {
            service: row.get(0)?,
            host: row.get(1)?,
            updated_at: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn delete_token(conn: &Connection, service: &str) -> rusqlite::Result<usize> {
    let host = local_host();
    conn.execute(
        "DELETE FROM ipc_auth_tokens WHERE service=?1 AND host=?2",
        params![service, host],
    )
}

// --- T8048: CRDT sync integration ---

#[derive(Debug, Serialize)]
pub struct TokenSyncHealth {
    pub total_tokens: usize,
    pub hosts_with_tokens: usize,
    pub services: Vec<String>,
}

pub fn check_token_sync_health(conn: &Connection) -> rusqlite::Result<TokenSyncHealth> {
    let total: usize = conn.query_row("SELECT count(*) FROM ipc_auth_tokens", [], |r| r.get(0))?;
    let hosts: usize = conn.query_row(
        "SELECT count(DISTINCT host) FROM ipc_auth_tokens",
        [],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare("SELECT DISTINCT service FROM ipc_auth_tokens ORDER BY service")?;
    let services: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(TokenSyncHealth {
        total_tokens: total,
        hosts_with_tokens: hosts,
        services,
    })
}

pub fn sync_tokens_from_peer(
    conn: &Connection,
    peer_tokens: &[(String, Vec<u8>, Vec<u8>, String, String)],
) -> rusqlite::Result<usize> {
    let mut count = 0;
    for (service, encrypted, nonce, host, updated_at) in peer_tokens {
        conn.execute(
            "INSERT OR REPLACE INTO ipc_auth_tokens (service, encrypted_token, nonce, host, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![service, encrypted, nonce, host, updated_at],
        )?;
        count += 1;
    }
    Ok(count)
}

// --- T8050: Token revocation ---

pub fn revoke_token(conn: &Connection, service: &str, host: &str) -> rusqlite::Result<usize> {
    warn!(service, host, "revoking token");
    conn.execute(
        "DELETE FROM ipc_auth_tokens WHERE service=?1 AND host=?2",
        params![service, host],
    )
}

// --- T8051: Key rotation ---

pub fn rotate_keys(
    conn: &Connection,
    old_secret: &str,
    new_secret: &str,
) -> rusqlite::Result<usize> {
    info!("rotating encryption keys");
    let host = local_host();
    let old_key = derive_key(old_secret);
    let new_key = derive_key(new_secret);
    let mut stmt =
        conn.prepare("SELECT service, encrypted_token, nonce FROM ipc_auth_tokens WHERE host=?1")?;
    let tokens: Vec<(String, Vec<u8>, Vec<u8>)> = stmt
        .query_map(params![host], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut rotated = 0;
    for (service, encrypted, nonce_vec) in &tokens {
        let nonce: [u8; 12] = match nonce_vec.clone().try_into() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let plaintext = match decrypt_token(&old_key, encrypted, &nonce) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let (new_encrypted, new_nonce) = encrypt_token(&new_key, &plaintext);
        conn.execute(
            "UPDATE ipc_auth_tokens SET encrypted_token=?1, nonce=?2, updated_at=datetime('now')
             WHERE service=?3 AND host=?4",
            params![new_encrypted, new_nonce.to_vec(), service, host],
        )?;
        rotated += 1;
    }
    Ok(rotated)
}

pub fn local_host() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}
