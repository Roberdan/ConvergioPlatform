use std::path::PathBuf;
use std::sync::Arc;

use super::engine::IpcEngine;
use super::protocol::{
    decode_request, encode_response, read_ipc_frame, write_ipc_frame, IpcResponse,
};

#[cfg(unix)]
pub async fn start_ipc_server(engine: Arc<IpcEngine>, socket_path: PathBuf) -> Result<(), String> {
    use tokio::net::UnixListener;
    // Remove stale socket
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).ok();
    }

    // Ensure parent directory exists
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }

    let listener = UnixListener::bind(&socket_path)
        .map_err(|e| format!("bind {}: {e}", socket_path.display()))?;

    // Set socket permissions to 0600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&socket_path, perms).map_err(|e| format!("chmod: {e}"))?;
    }

    tracing::info!("IPC server listening on {}", socket_path.display());

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let eng = engine.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, eng).await {
                        tracing::warn!("IPC client error: {e}");
                    }
                });
            }
            Err(e) => {
                tracing::warn!("IPC accept error: {e}");
            }
        }
    }
}

#[cfg(unix)]
async fn handle_client(
    stream: tokio::net::UnixStream,
    engine: Arc<IpcEngine>,
) -> Result<(), String> {
    let (mut reader, mut writer) = stream.into_split();

    loop {
        let frame = match read_ipc_frame(&mut reader).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(format!("read frame: {e}")),
        };

        let request = decode_request(&frame).map_err(|e| format!("decode: {e}"))?;

        let needs_async = matches!(
            &request,
            super::protocol::IpcRequest::Receive { wait: true, .. }
        );

        let response = if needs_async {
            engine
                .dispatch(request)
                .await
                .unwrap_or_else(|e| IpcResponse::Error {
                    code: 500,
                    message: e.to_string(),
                })
        } else {
            let eng = engine.clone();
            tokio::task::spawn_blocking(move || {
                tokio::runtime::Handle::current().block_on(eng.dispatch(request))
            })
            .await
            .map_err(|e| format!("spawn_blocking: {e}"))?
            .unwrap_or_else(|e| IpcResponse::Error {
                code: 500,
                message: e.to_string(),
            })
        };

        let resp_bytes = encode_response(&response).map_err(|e| format!("encode: {e}"))?;
        write_ipc_frame(&mut writer, &resp_bytes)
            .await
            .map_err(|e| format!("write frame: {e}"))?;
    }
}

#[cfg(not(unix))]
pub async fn start_ipc_server(
    _engine: Arc<IpcEngine>,
    _socket_path: PathBuf,
) -> Result<(), String> {
    Err("IPC Unix socket server not supported on this platform. Use TCP fallback.".into())
}
