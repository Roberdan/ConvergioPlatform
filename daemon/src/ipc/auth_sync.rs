use chacha20poly1305::aead::rand_core::RngCore;
use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use hkdf::Hkdf;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::path::PathBuf;
use tracing::{info, warn};

// --- T8046: Encryption core ---

pub fn derive_key(shared_secret: &str) -> [u8; 32] {
    tracing::info!("deriving key from shared secret");
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_default();
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    let salt = format!("{hostname}:{username}");
    let hk = Hkdf::<Sha256>::new(Some(salt.as_bytes()), shared_secret.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(b"ipc-auth-v1", &mut okm)
        .expect("HKDF expand failed");
    okm
}

pub fn encrypt_token(key: &[u8; 32], plaintext: &str) -> (Vec<u8>, [u8; 12]) {
    tracing::info!("encrypting token");
    let cipher = ChaCha20Poly1305::new(key.into());
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("encryption failed");
    (ciphertext, nonce_bytes)
}

pub fn decrypt_token(
    key: &[u8; 32],
    ciphertext: &[u8],
    nonce: &[u8; 12],
) -> Result<String, String> {
    tracing::info!("decrypting token");
    let cipher = ChaCha20Poly1305::new(key.into());
    let n = Nonce::from_slice(nonce);
    let plaintext = cipher
        .decrypt(n, ciphertext)
        .map_err(|e| format!("decryption failed: {e}"))?;
    String::from_utf8(plaintext).map_err(|e| format!("invalid UTF-8: {e}"))
}

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

// --- T8049: File watcher ---

#[derive(Debug, Clone)]
pub struct CredentialChange {
    pub path: PathBuf,
    pub service: String,
}

const CREDENTIAL_WATCH_PATHS: &[(&str, &str)] = &[
    (".claude/.credentials", "claude"),
    (".config/gh/hosts.yml", "gh"),
    (".config/opencode/config.json", "opencode"),
];

pub fn credential_watch_paths() -> Vec<(PathBuf, String)> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let mut paths: Vec<(PathBuf, String)> = CREDENTIAL_WATCH_PATHS
        .iter()
        .map(|(p, s)| (PathBuf::from(&home).join(p), s.to_string()))
        .collect();
    if let Ok(extra) = std::env::var("CREDENTIAL_WATCH_PATHS") {
        for entry in extra.split(':') {
            if let Some((path, svc)) = entry.split_once('=') {
                paths.push((PathBuf::from(path), svc.to_string()));
            }
        }
    }
    // OS-aware: macOS Keychain path
    #[cfg(target_os = "macos")]
    paths.push((
        PathBuf::from(&home).join("Library/Keychains"),
        "keychain".to_string(),
    ));
    // Linux XDG paths
    #[cfg(target_os = "linux")]
    {
        let xdg = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{home}/.config"));
        paths.push((PathBuf::from(&xdg).join("credentials"), "xdg".to_string()));
    }
    paths
}

pub fn watch_credential_files(
    tx: tokio::sync::mpsc::Sender<CredentialChange>,
) -> Result<(), String> {
    use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    let paths = credential_watch_paths();
    let tx_clone = tx.clone();
    let path_map: std::collections::HashMap<PathBuf, String> = paths.iter().cloned().collect();
    let map = path_map.clone();
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                ) {
                    for path in &event.paths {
                        for (watch_path, svc) in &map {
                            if path.starts_with(watch_path) || path == watch_path {
                                let _ = tx_clone.blocking_send(CredentialChange {
                                    path: path.clone(),
                                    service: svc.clone(),
                                });
                            }
                        }
                    }
                }
            }
        },
        Config::default(),
    )
    .map_err(|e| format!("watcher init: {e}"))?;

    for (path, _) in &paths {
        if path.exists() {
            let _ = watcher.watch(path, RecursiveMode::NonRecursive);
        }
    }
    // Leak the watcher so it keeps running
    std::mem::forget(watcher);
    Ok(())
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

fn local_host() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE ipc_auth_tokens (id INTEGER PRIMARY KEY, service TEXT NOT NULL, encrypted_token BLOB NOT NULL, nonce BLOB NOT NULL, host TEXT NOT NULL DEFAULT '', updated_at TEXT NOT NULL DEFAULT '', UNIQUE (service, host));").unwrap();
        conn
    }

    #[test]
    fn test_derive_key_consistent() {
        let k1 = derive_key("secret123");
        let k2 = derive_key("secret123");
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_derive_key_different_secrets() {
        let k1 = derive_key("secret1");
        let k2 = derive_key("secret2");
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = derive_key("test-secret");
        let (ct, nonce) = encrypt_token(&key, "my-token-value");
        let pt = decrypt_token(&key, &ct, &nonce).unwrap();
        assert_eq!(pt, "my-token-value");
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let key1 = derive_key("right-key");
        let key2 = derive_key("wrong-key");
        let (ct, nonce) = encrypt_token(&key1, "secret");
        assert!(decrypt_token(&key2, &ct, &nonce).is_err());
    }

    #[test]
    fn test_store_get_roundtrip() {
        let conn = setup_db();
        store_token(&conn, "claude", "tok-123", "secret").unwrap();
        let val = get_token(&conn, "claude", "secret").unwrap();
        assert_eq!(val, Some("tok-123".to_string()));
    }

    #[test]
    fn test_revoke_token() {
        let conn = setup_db();
        store_token(&conn, "gh", "ghp_abc", "s").unwrap();
        let host = local_host();
        let n = revoke_token(&conn, "gh", &host).unwrap();
        assert_eq!(n, 1);
        assert_eq!(get_token(&conn, "gh", "s").unwrap(), None);
    }

    #[test]
    fn test_rotate_keys() {
        let conn = setup_db();
        store_token(&conn, "svc", "val", "old").unwrap();
        let n = rotate_keys(&conn, "old", "new").unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            get_token(&conn, "svc", "new").unwrap(),
            Some("val".to_string())
        );
        assert_eq!(get_token(&conn, "svc", "old").unwrap(), None);
    }

    #[test]
    fn test_list_tokens() {
        let conn = setup_db();
        store_token(&conn, "a", "t1", "s").unwrap();
        store_token(&conn, "b", "t2", "s").unwrap();
        let list = list_tokens(&conn).unwrap();
        assert_eq!(list.len(), 2);
    }
}
