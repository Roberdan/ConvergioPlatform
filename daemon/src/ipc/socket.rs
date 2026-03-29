use std::path::PathBuf;
use std::sync::Arc;

use super::engine::IpcEngine;
use super::error::IpcError;
use super::protocol::{
    decode_request, encode_response, read_ipc_frame, write_ipc_frame, IpcResponse,
};

#[cfg(unix)]
pub async fn start_ipc_server(
    engine: Arc<IpcEngine>,
    socket_path: PathBuf,
) -> Result<(), IpcError> {
    use tokio::net::UnixListener;
    // Remove stale socket
    if socket_path.exists() {
        if let Err(e) = std::fs::remove_file(&socket_path) {
            tracing::debug!("cleanup: remove stale socket: {e}");
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(&socket_path)?;

    // Set socket permissions to 0600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&socket_path, perms)?;
    }

    tracing::info!("IPC server listening on {}", socket_path.display());

    loop { // UNBOUNDED: event loop
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
) -> Result<(), IpcError> {
    let (mut reader, mut writer) = stream.into_split();

    loop { // UNBOUNDED: event loop
        let frame = match read_ipc_frame(&mut reader).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(IpcError::Io(e)),
        };

        let request = decode_request(&frame)?;

        // dispatch is async — await directly for all request types
        let response = engine
            .dispatch(request)
            .await
            .unwrap_or_else(|e| IpcResponse::Error {
                code: 500,
                message: e.to_string(),
            });

        let resp_bytes = encode_response(&response)?;
        write_ipc_frame(&mut writer, &resp_bytes).await?;
    }
}

#[cfg(not(unix))]
pub async fn start_ipc_server(
    _engine: Arc<IpcEngine>,
    _socket_path: PathBuf,
) -> Result<(), IpcError> {
    Err(IpcError::Other(
        "IPC Unix socket server not supported on this platform. Use TCP fallback.".into(),
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::super::engine::IpcEngine;
    use super::super::protocol::{IpcRequest, IpcResponse};

    fn temp_engine() -> (Arc<IpcEngine>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("test-ipc.db");
        (Arc::new(IpcEngine::new(db)), dir)
    }

    /// Verify IPC dispatch path works via direct .await — no blocking workaround.
    /// Tests that non-waiting requests (e.g. Ping) are dispatched correctly.
    #[tokio::test]
    async fn test_dispatch_ping() {
        let (engine, _dir) = temp_engine();
        let resp = engine.dispatch(IpcRequest::Ping).await.unwrap();
        assert!(
            matches!(resp, IpcResponse::Pong { .. }),
            "expected Pong, got {resp:?}"
        );
    }

    /// Verify IPC dispatch handles non-waiting Receive via direct await.
    /// This path previously used an incorrect blocking workaround.
    #[tokio::test]
    async fn test_dispatch_receive_no_wait() {
        let (engine, _dir) = temp_engine();
        let resp = engine
            .dispatch(IpcRequest::Receive {
                agent: "bob".into(),
                from_filter: None,
                channel_filter: None,
                limit: 10,
                peek: false,
                wait: false,
            })
            .await
            .unwrap();
        assert!(
            matches!(resp, IpcResponse::MessageList { .. }),
            "expected MessageList, got {resp:?}"
        );
    }

    /// Verify IPC dispatch path handles Register+Send+Receive round-trip.
    #[tokio::test]
    async fn test_dispatch_send_receive_round_trip() {
        let (engine, _dir) = temp_engine();

        engine
            .dispatch(IpcRequest::Register {
                name: "alice".into(),
                agent_type: "claude".into(),
                pid: None,
                host: "local".into(),
                metadata: None,
            })
            .await
            .unwrap();

        engine
            .dispatch(IpcRequest::Send {
                from: "alice".into(),
                to: "bob".into(),
                content: "hello".into(),
                msg_type: "text".into(),
                priority: 0,
            })
            .await
            .unwrap();

        let resp = engine
            .dispatch(IpcRequest::Receive {
                agent: "bob".into(),
                from_filter: None,
                channel_filter: None,
                limit: 10,
                peek: false,
                wait: false,
            })
            .await
            .unwrap();

        match resp {
            IpcResponse::MessageList { messages } => {
                assert_eq!(messages.len(), 1);
                assert_eq!(messages[0].content, "hello");
            }
            _ => panic!("expected MessageList"),
        }
    }
}
