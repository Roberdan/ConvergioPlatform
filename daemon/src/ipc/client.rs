use std::path::Path;
use std::time::{Duration, Instant};

use super::engine::IpcEngine;
use super::protocol::{IpcRequest, IpcResponse};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

#[cfg(unix)]
pub async fn ipc_request(socket_path: &Path, request: &IpcRequest) -> Result<IpcResponse, String> {
    use super::protocol::{decode_response, encode_request, read_ipc_frame, write_ipc_frame};
    use tokio::net::UnixStream;
    use tokio::time::timeout;

    let stream = timeout(CONNECT_TIMEOUT, UnixStream::connect(socket_path))
        .await
        .map_err(|_| "connection timeout".to_string())?
        .map_err(|e| format!("connect: {e}"))?;

    let (mut reader, mut writer) = stream.into_split();

    let req_bytes = encode_request(request).map_err(|e| format!("encode: {e}"))?;
    write_ipc_frame(&mut writer, &req_bytes)
        .await
        .map_err(|e| format!("write: {e}"))?;

    let resp_frame = read_ipc_frame(&mut reader)
        .await
        .map_err(|e| format!("read: {e}"))?;

    decode_response(&resp_frame).map_err(|e| format!("decode: {e}"))
}

#[cfg(not(unix))]
pub async fn ipc_request(
    _socket_path: &Path,
    _request: &IpcRequest,
) -> Result<IpcResponse, String> {
    Err("Unix socket IPC not supported on this platform".into())
}

pub fn ipc_request_direct(db_path: &Path, request: &IpcRequest) -> Result<IpcResponse, String> {
    let engine = IpcEngine::new(db_path.to_path_buf());
    // Ensure WAL mode + busy_timeout for direct access
    let conn = engine.open_conn().map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| e.to_string())?;
    drop(conn);
    let rt = tokio::runtime::Handle::current();
    rt.block_on(engine.dispatch(request.clone()))
        .map_err(|e| e.to_string())
}

pub async fn ipc_request_with_fallback(
    socket_path: &Path,
    db_path: &Path,
    request: &IpcRequest,
) -> Result<IpcResponse, String> {
    let start = Instant::now();
    match ipc_request(socket_path, request).await {
        Ok(resp) => Ok(resp),
        Err(socket_err) => {
            let elapsed = start.elapsed();
            tracing::warn!(
                "daemon unavailable, using direct SQLite fallback ({elapsed:?}): {socket_err}"
            );
            let db = db_path.to_path_buf();
            let req = request.clone();
            let engine = IpcEngine::new(db);
            engine.dispatch(req).await.map_err(|e| e.to_string())
        }
    }
}
