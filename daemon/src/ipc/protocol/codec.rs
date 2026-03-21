use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::messages::{IpcRequest, IpcResponse, IPC_PROTOCOL_VERSION};

const MAX_FRAME_SIZE: u32 = 4 * 1024 * 1024; // 4 MB

// ── Wire I/O ─────────────────────────────────────────────

pub async fn write_ipc_frame<W: AsyncWriteExt + Unpin>(
    w: &mut W,
    payload: &[u8],
) -> std::io::Result<()> {
    let len = payload.len() as u32;
    if len > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("frame too large: {len} > {MAX_FRAME_SIZE}"),
        ));
    }
    w.write_u8(IPC_PROTOCOL_VERSION).await?;
    w.write_u32(len).await?;
    w.write_all(payload).await?;
    w.flush().await
}

pub async fn read_ipc_frame<R: AsyncReadExt + Unpin>(r: &mut R) -> std::io::Result<Vec<u8>> {
    let version = r.read_u8().await?;
    if version != IPC_PROTOCOL_VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "unsupported IPC protocol version: {version:#04x}, expected {IPC_PROTOCOL_VERSION:#04x}"
            ),
        ));
    }
    let len = r.read_u32().await?;
    if len > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("frame too large: {len} > {MAX_FRAME_SIZE}"),
        ));
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf).await?;
    Ok(buf)
}

pub fn encode_request(req: &IpcRequest) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    rmp_serde::to_vec_named(req)
}

pub fn decode_request(data: &[u8]) -> Result<IpcRequest, rmp_serde::decode::Error> {
    rmp_serde::from_slice(data)
}

pub fn encode_response(resp: &IpcResponse) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    rmp_serde::to_vec_named(resp)
}

pub fn decode_response(data: &[u8]) -> Result<IpcResponse, rmp_serde::decode::Error> {
    rmp_serde::from_slice(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_roundtrip() {
        let req = IpcRequest::Register {
            name: "planner".into(),
            agent_type: "claude".into(),
            pid: Some(1234),
            host: "mac-worker-2".into(),
            metadata: None,
        };
        let encoded = encode_request(&req).unwrap();
        let decoded = decode_request(&encoded).unwrap();
        match decoded {
            IpcRequest::Register { name, host, .. } => {
                assert_eq!(name, "planner");
                assert_eq!(host, "mac-worker-2");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_response_roundtrip() {
        let resp = IpcResponse::Pong { uptime_secs: 42 };
        let encoded = encode_response(&resp).unwrap();
        let decoded = decode_response(&encoded).unwrap();
        match decoded {
            IpcResponse::Pong { uptime_secs } => assert_eq!(uptime_secs, 42),
            _ => panic!("wrong variant"),
        }
    }

    #[tokio::test]
    async fn test_frame_roundtrip() {
        let payload = b"hello ipc";
        let mut buf = Vec::new();
        write_ipc_frame(&mut buf, payload).await.unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let read_back = read_ipc_frame(&mut cursor).await.unwrap();
        assert_eq!(read_back, payload);
    }

    #[tokio::test]
    async fn test_frame_rejects_bad_version() {
        let mut buf = vec![0x99, 0, 0, 0, 1, 0x42]; // version 0x99
        let mut cursor = std::io::Cursor::new(&mut buf);
        let err = read_ipc_frame(&mut cursor).await.unwrap_err();
        assert!(err.to_string().contains("unsupported IPC protocol version"));
    }

    #[tokio::test]
    async fn test_protocol_version_reject() {
        // Version 0x02 should be rejected
        let mut buf = vec![0x02, 0, 0, 0, 1, 0x00];
        let mut cursor = std::io::Cursor::new(&mut buf);
        let err = read_ipc_frame(&mut cursor).await.unwrap_err();
        assert!(err.to_string().contains("unsupported IPC protocol version"));
        assert!(err.to_string().contains("0x02"));
    }
}
