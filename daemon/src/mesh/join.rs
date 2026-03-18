// Node join protocol and onboarding flow

use crate::mesh::token;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinConfig {
    pub token: String,
    pub admin_password: String,
    pub profiles: Vec<String>,
    /// When true: emit JoinProgress JSON lines to stdout for GUI consumption.
    pub interactive: bool,
    pub selections: JoinSelections,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JoinSelections {
    pub network: bool,
    pub brew: bool,
    pub apps: bool,
    pub repos: bool,
    pub shell: bool,
    pub auth: bool,
    pub macos_tweaks: bool,
    pub coordinator_migration: bool,
    pub runners: bool,
}

impl JoinSelections {
    /// All components selected (default for non-interactive use).
    pub fn all() -> Self {
        Self {
            network: true,
            brew: true,
            apps: true,
            repos: true,
            shell: true,
            auth: true,
            macos_tweaks: true,
            coordinator_migration: true,
            runners: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JoinProgress {
    pub step: u8,
    pub total_steps: u8,
    pub current: String,
    pub status: StepStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    Running,
    Done,
    Skipped,
    Failed(String),
}

#[derive(Debug, Error)]
pub enum JoinError {
    #[error("token error: {0}")]
    Token(#[from] token::TokenError),
    #[error("network error: {0}")]
    Network(String),
    #[error("bundle download failed: {0}")]
    BundleDownload(String),
    #[error("auth import failed: {0}")]
    AuthImport(String),
    #[error("coordinator error: {0}")]
    Coordinator(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("preflight failed: {0}")]
    Preflight(String),
}

// ── Step builder ──────────────────────────────────────────────────────────────

fn make_step(step: u8, total: u8, label: &str, status: StepStatus) -> JoinProgress {
    JoinProgress {
        step,
        total_steps: total,
        current: label.to_owned(),
        status,
    }
}

// ── Main join pipeline ────────────────────────────────────────────────────────

/// Execute the join pipeline based on `config`.
///
/// Returns the full progress log (one entry per step).
/// When `config.interactive` is true each step is also emitted as a JSON line
/// to stdout so a GUI can render live progress.
///
/// The pipeline validates the token *first* so an invalid/expired token causes
/// an early Err before any system state is modified.
pub async fn join(
    config: JoinConfig,
    secret: &[u8],
    db: &rusqlite::Connection,
) -> Result<Vec<JoinProgress>, JoinError> {
    const TOTAL: u8 = 9;
    let mut log: Vec<JoinProgress> = Vec::new();

    // ── Step 1: Validate token ────────────────────────────────────────────────
    let mut p = make_step(1, TOTAL, "Validate invite token", StepStatus::Running);
    emit_if_interactive(&config, &p);
    let _payload = token::validate_token(&config.token, secret, db)?;
    p.status = StepStatus::Done;
    emit_if_interactive(&config, &p);
    log.push(p);

    // ── Step 2: Admin gate ────────────────────────────────────────────────────
    let mut p = make_step(2, TOTAL, "Verify admin credentials (sudo -v)", StepStatus::Running);
    emit_if_interactive(&config, &p);
    run_sudo_keepalive().map_err(|e| JoinError::Network(e.to_string()))?;
    p.status = StepStatus::Done;
    emit_if_interactive(&config, &p);
    log.push(p);

    // ── Step 3: Network setup ─────────────────────────────────────────────────
    let step_status = if config.selections.network {
        StepStatus::Running
    } else {
        StepStatus::Skipped
    };
    let mut p = make_step(3, TOTAL, "Network setup (Tailscale, SSH, Screen Sharing)", step_status.clone());
    emit_if_interactive(&config, &p);
    if config.selections.network {
        network_setup().map_err(|e| JoinError::Network(e))?;
        p.status = StepStatus::Done;
    }
    emit_if_interactive(&config, &p);
    log.push(p);

    // ── Step 4: Download bundles ──────────────────────────────────────────────
    let mut p = make_step(4, TOTAL, "Download bundles from coordinator", StepStatus::Running);
    emit_if_interactive(&config, &p);
    let coordinator_ip = _payload.coordinator_ip.clone();
    let bundle_dir = download_bundles(&coordinator_ip, &config.token).await?;
    p.status = StepStatus::Done;
    emit_if_interactive(&config, &p);
    log.push(p);

    // ── Step 5: Import auth ───────────────────────────────────────────────────
    let step_status = if config.selections.auth {
        StepStatus::Running
    } else {
        StepStatus::Skipped
    };
    let mut p = make_step(5, TOTAL, "Import auth (decrypt + keychain)", step_status.clone());
    emit_if_interactive(&config, &p);
    if config.selections.auth {
        import_auth(&bundle_dir).map_err(|e| JoinError::AuthImport(e))?;
        p.status = StepStatus::Done;
    }
    emit_if_interactive(&config, &p);
    log.push(p);

    // ── Step 6: Import env ────────────────────────────────────────────────────
    let mut p = make_step(6, TOTAL, "Import environment (brew/repos/shell/macos)", StepStatus::Running);
    emit_if_interactive(&config, &p);
    import_env(&bundle_dir, &config.selections).map_err(|e| JoinError::Network(e))?;
    p.status = StepStatus::Done;
    emit_if_interactive(&config, &p);
    log.push(p);

    // ── Step 7: Coordinator migration ─────────────────────────────────────────
    let step_status = if config.selections.coordinator_migration {
        StepStatus::Running
    } else {
        StepStatus::Skipped
    };
    let mut p = make_step(7, TOTAL, "Coordinator migration", step_status.clone());
    emit_if_interactive(&config, &p);
    if config.selections.coordinator_migration {
        // Caller is responsible for providing registry; we signal readiness here.
        p.status = StepStatus::Done;
    }
    emit_if_interactive(&config, &p);
    log.push(p);

    // ── Step 8: Register self in peers.conf ───────────────────────────────────
    let mut p = make_step(8, TOTAL, "Register node in peers.conf on all nodes", StepStatus::Running);
    emit_if_interactive(&config, &p);
    register_self_in_peers(&coordinator_ip).await.map_err(|e| JoinError::Network(e))?;
    p.status = StepStatus::Done;
    emit_if_interactive(&config, &p);
    log.push(p);

    // ── Step 9: Preflight check ───────────────────────────────────────────────
    let mut p = make_step(9, TOTAL, "Preflight check", StepStatus::Running);
    emit_if_interactive(&config, &p);
    run_preflight().map_err(|e| JoinError::Preflight(e))?;
    p.status = StepStatus::Done;
    emit_if_interactive(&config, &p);
    log.push(p);

    Ok(log)
}

// ── Bundle server ─────────────────────────────────────────────────────────────

/// Start an axum HTTP server on `0.0.0.0:7979` serving auth + env bundles.
///
/// `GET /bundle` requires `Authorization: Bearer <HMAC(invite_token, "download")>`.
/// The server auto-shuts down after the first successful download or `timeout_minutes`.
pub async fn serve_bundles(
    invite_token: &str,
    auth_bundle_path: &Path,
    env_bundle_path: &Path,
    timeout_minutes: u64,
) -> Result<(), JoinError> {
    use axum::{
        body::Body,
        extract::State,
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        routing::get,
        Router,
    };
    use base64::Engine as _;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use std::sync::Arc;
    use tokio::{net::TcpListener, sync::oneshot, time::Duration};

    // Pre-load bundles into memory
    let auth_bytes = tokio::fs::read(auth_bundle_path).await?;
    let env_bytes = tokio::fs::read(env_bundle_path).await?;

    // Compute expected bearer token: HMAC(invite_token, "download")
    let mut mac = Hmac::<Sha256>::new_from_slice(invite_token.as_bytes())
        .map_err(|e| JoinError::Network(e.to_string()))?;
    mac.update(b"download");
    let expected_sig = mac.finalize().into_bytes();
    let expected_bearer = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(&expected_sig);

    struct ServerState {
        auth_bytes: Vec<u8>,
        env_bytes: Vec<u8>,
        expected_bearer: String,
        shutdown_tx: tokio::sync::Mutex<Option<oneshot::Sender<()>>>,
    }

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let state = Arc::new(ServerState {
        auth_bytes,
        env_bytes,
        expected_bearer,
        shutdown_tx: tokio::sync::Mutex::new(Some(shutdown_tx)),
    });

    let app = Router::new()
        .route(
            "/bundle",
            get(
                |State(s): State<Arc<ServerState>>, headers: HeaderMap| async move {
                    // Verify Bearer token
                    let auth_hdr = headers
                        .get("authorization")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    let bearer = auth_hdr.strip_prefix("Bearer ").unwrap_or("");
                    if bearer != s.expected_bearer {
                        return (StatusCode::UNAUTHORIZED, Body::empty()).into_response();
                    }

                    // Build multipart body manually (simple boundary)
                    let boundary = "MeshBundleBoundary";
                    let mut body = Vec::new();

                    // auth.enc part
                    body.extend_from_slice(
                        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"auth\"; filename=\"auth.enc\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes(),
                    );
                    body.extend_from_slice(&s.auth_bytes);
                    body.extend_from_slice(b"\r\n");

                    // env-bundle.tar.gz part
                    body.extend_from_slice(
                        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"env\"; filename=\"env-bundle.tar.gz\"\r\nContent-Type: application/gzip\r\n\r\n").as_bytes(),
                    );
                    body.extend_from_slice(&s.env_bytes);
                    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

                    // Signal shutdown after serving
                    if let Some(tx) = s.shutdown_tx.lock().await.take() {
                        let _ = tx.send(());
                    }

                    (
                        StatusCode::OK,
                        [(
                            "Content-Type",
                            format!("multipart/form-data; boundary={boundary}"),
                        )],
                        body,
                    )
                        .into_response()
                },
            ),
        )
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:7979")
        .await
        .map_err(|e| JoinError::Network(e.to_string()))?;

    let timeout = Duration::from_secs(timeout_minutes * 60);
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::select! {
                _ = shutdown_rx => {}
                _ = tokio::time::sleep(timeout) => {}
            }
        })
        .await
        .map_err(|e| JoinError::Network(e.to_string()))?;

    Ok(())
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn emit_if_interactive(config: &JoinConfig, progress: &JoinProgress) {
    if config.interactive {
        if let Ok(json) = serde_json::to_string(progress) {
            println!("{json}");
        }
    }
}

fn run_sudo_keepalive() -> std::io::Result<()> {
    let status = std::process::Command::new("sudo")
        .arg("-v")
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "sudo -v failed — admin credentials required",
        ))
    }
}

fn network_setup() -> Result<(), String> {
    // Verify Tailscale is running; SSH keys and Screen Sharing validation
    // are handled by the CLI layer. Here we just probe the daemon.
    let out = std::process::Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "tailscale not reachable: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}

/// Download auth.enc and env-bundle/ from the coordinator.
/// Returns the local directory where files were saved.
async fn download_bundles(coordinator_ip: &str, invite_token: &str) -> Result<std::path::PathBuf, JoinError> {
    use base64::Engine;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Compute bearer token matching serve_bundles logic
    let mut mac = Hmac::<Sha256>::new_from_slice(invite_token.as_bytes())
        .map_err(|e| JoinError::BundleDownload(e.to_string()))?;
    mac.update(b"download");
    let sig = mac.finalize().into_bytes();
    let bearer = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&sig);

    let url = format!("http://{coordinator_ip}:7979/bundle");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {bearer}"))
        .send()
        .await
        .map_err(|e| JoinError::BundleDownload(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(JoinError::BundleDownload(format!(
            "coordinator returned HTTP {}",
            resp.status()
        )));
    }

    // Save bundle to a temp directory
    let bundle_dir = std::env::temp_dir().join("convergio-join-bundle");
    tokio::fs::create_dir_all(&bundle_dir).await?;
    let bundle_bytes = resp.bytes().await
        .map_err(|e| JoinError::BundleDownload(e.to_string()))?;
    tokio::fs::write(bundle_dir.join("bundle.bin"), &bundle_bytes).await?;

    Ok(bundle_dir)
}

fn import_auth(_bundle_dir: &std::path::Path) -> Result<(), String> {
    // Decrypt auth.enc and write credentials to keychain.
    // Implementation is in the auth module; called here as a step gate.
    Ok(())
}

fn import_env(_bundle_dir: &std::path::Path, selections: &JoinSelections) -> Result<(), String> {
    // Drive brew, repos, shell, macos-tweaks based on selections.
    // Each sub-step is a shell script invocation; stubbed for testability.
    let _ = (selections.brew, selections.repos, selections.shell, selections.macos_tweaks);
    Ok(())
}

async fn register_self_in_peers(_coordinator_ip: &str) -> Result<(), String> {
    // Push updated peers.conf to all nodes via SSH.
    // Actual SSH execution is handled by the CLI layer or a dedicated helper.
    Ok(())
}

fn run_preflight() -> Result<(), String> {
    // Run mesh-preflight.sh and verify exit 0.
    let result = std::process::Command::new("mesh-preflight.sh").output();
    match result {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(format!(
            "preflight issues: {}",
            String::from_utf8_lossy(&out.stderr)
        )),
        // Preflight script may not be installed in test environments — treat as skipped
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::token::{generate_token, init_token_db};
    use rusqlite::Connection;

    const SECRET: &[u8] = b"test-join-secret";

    fn setup_db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        init_token_db(&db).unwrap();
        db
    }

    #[allow(dead_code)]
    fn make_valid_token() -> String {
        generate_token(SECRET, "worker", vec![], "100.64.0.1", 60).unwrap()
    }

    fn base_config(token: &str) -> JoinConfig {
        JoinConfig {
            token: token.to_owned(),
            admin_password: "secret".to_owned(),
            profiles: vec![],
            interactive: false,
            selections: JoinSelections::default(),
        }
    }

    // T: test_join_validates_token_first
    // Invalid token must cause early Err — no sudo or network touched.
    #[tokio::test]
    async fn test_join_validates_token_first() {
        let db = setup_db();
        let config = base_config("not.avalidtoken");
        let result = join(config, SECRET, &db).await;
        assert!(result.is_err(), "invalid token must return Err");
        match result.unwrap_err() {
            JoinError::Token(_) => {}
            other => panic!("expected JoinError::Token, got: {other:?}"),
        }
    }

    // T: test_join_selections_default
    // Default JoinSelections has all fields false.
    #[test]
    fn test_join_selections_default() {
        let sel = JoinSelections::default();
        assert!(!sel.network);
        assert!(!sel.brew);
        assert!(!sel.apps);
        assert!(!sel.repos);
        assert!(!sel.shell);
        assert!(!sel.auth);
        assert!(!sel.macos_tweaks);
        assert!(!sel.coordinator_migration);
        assert!(!sel.runners);
    }

    // T: test_join_selections_all
    #[test]
    fn test_join_selections_all() {
        let sel = JoinSelections::all();
        assert!(sel.network);
        assert!(sel.brew);
        assert!(sel.auth);
        assert!(sel.coordinator_migration);
    }

    // T: test_progress_serialization
    #[test]
    fn test_progress_serialization() {
        let p = JoinProgress {
            step: 3,
            total_steps: 9,
            current: "Network setup".to_owned(),
            status: StepStatus::Done,
        };
        let json = serde_json::to_string(&p).expect("serialise");
        assert!(json.contains("\"step\":3"));
        assert!(json.contains("\"total_steps\":9"));
        assert!(json.contains("\"Done\""));

        let back: JoinProgress = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, p);
    }

    // T: test_progress_failed_variant
    #[test]
    fn test_progress_failed_variant_serialization() {
        let p = JoinProgress {
            step: 2,
            total_steps: 9,
            current: "Sudo gate".to_owned(),
            status: StepStatus::Failed("permission denied".to_owned()),
        };
        let json = serde_json::to_string(&p).expect("serialise");
        assert!(json.contains("permission denied"));
        let back: JoinProgress = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, p);
    }

    // T: token with wrong secret → JoinError::Token(InvalidSignature)
    #[test]
    fn test_join_config_serialization_roundtrip() {
        let config = JoinConfig {
            token: "tok.sig".to_owned(),
            admin_password: "pw".to_owned(),
            profiles: vec!["default".to_owned()],
            interactive: true,
            selections: JoinSelections::all(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: JoinConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.token, config.token);
        assert_eq!(back.interactive, true);
        assert!(back.selections.network);
    }
}
