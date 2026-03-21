use chacha20poly1305::aead::rand_core::RngCore;
use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;
use std::path::PathBuf;

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
