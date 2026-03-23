use std::io::ErrorKind;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

use super::types::{FramedMeshSyncFrame, MeshSyncFrame, PeerQuota, MAX_FRAME_BYTES};
use crate::mesh::error::MeshError;

pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    frame: &MeshSyncFrame,
) -> Result<(), MeshError> {
    let payload = rmp_serde::to_vec_named(frame)?;
    let len = u32::try_from(payload.len())
        .map_err(|_| MeshError::Serialization("mesh frame too large".into()))?;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&payload).await?;
    Ok(())
}

pub async fn read_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Option<MeshSyncFrame>, MeshError> {
    let mut quota = PeerQuota::new();
    match read_frame_with_quota(reader, &mut quota).await? {
        Some(framed) => {
            quota.release(framed.payload_len as usize);
            Ok(Some(framed.frame))
        }
        None => Ok(None),
    }
}

pub async fn read_frame_with_quota<R: AsyncRead + Unpin>(
    reader: &mut R,
    quota: &mut PeerQuota,
) -> Result<Option<FramedMeshSyncFrame>, MeshError> {
    let mut len_buf = [0_u8; 4];
    match reader.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(err) if err.kind() == ErrorKind::UnexpectedEof => return Ok(None),
        Err(err) => return Err(MeshError::from(err)),
    }
    let payload_len = u32::from_be_bytes(len_buf);
    if payload_len > MAX_FRAME_BYTES {
        return Err(MeshError::Network(format!(
            "mesh frame exceeds limit: {payload_len}"
        )));
    }
    quota.reserve(payload_len as usize)?;
    let payload = match read_payload_streaming(reader, payload_len as usize).await {
        Ok(payload) => payload,
        Err(err) => {
            quota.release(payload_len as usize);
            return Err(err);
        }
    };
    let frame = match rmp_serde::from_slice::<MeshSyncFrame>(&payload) {
        Ok(frame) => frame,
        Err(err) => {
            quota.release(payload_len as usize);
            return Err(MeshError::from(err));
        }
    };
    Ok(Some(FramedMeshSyncFrame { frame, payload_len }))
}

async fn read_payload_streaming<R: AsyncRead + Unpin>(
    reader: &mut R,
    payload_len: usize,
) -> Result<Vec<u8>, MeshError> {
    let mut payload = Vec::with_capacity(payload_len.min(64 * 1024));
    let mut limited = reader.take(payload_len as u64);
    let mut buffered = BufReader::new(&mut limited);
    let mut chunk = [0_u8; 8 * 1024];
    loop {
        let read = buffered.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        payload.extend_from_slice(&chunk[..read]);
    }
    if payload.len() != payload_len {
        return Err(MeshError::Network(format!(
            "mesh frame truncated: read {} of {payload_len}",
            payload.len()
        )));
    }
    Ok(payload)
}
