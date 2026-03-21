// Bundle server: serve and download auth+env bundles over HTTP

use super::types::JoinError;
use std::path::Path;

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
    let expected_bearer = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&expected_sig);

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

/// Download auth.enc and env-bundle/ from the coordinator.
/// Returns the local directory where files were saved.
pub(super) async fn download_bundles(
    coordinator_ip: &str,
    invite_token: &str,
) -> Result<std::path::PathBuf, JoinError> {
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
    let bundle_bytes = resp
        .bytes()
        .await
        .map_err(|e| JoinError::BundleDownload(e.to_string()))?;
    tokio::fs::write(bundle_dir.join("bundle.bin"), &bundle_bytes).await?;

    Ok(bundle_dir)
}
