// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Mesh audio routing — play_on_active_node: resolve target, dispatch local or remote.

use crate::kernel::audio::{resolve_active_node, play_local};
use tracing::{info, warn};

/// Play `audio` (raw WAV bytes) on the active node.
///
/// - If the active node is local: write to a temp file, exec `afplay`, delete.
/// - If the active node is remote: HTTP POST to `{base_url}/api/kernel/play`.
///
/// Errors are logged but never fatal — audio is best-effort.
pub async fn play_on_active_node(audio: &[u8], conn: &rusqlite::Connection) {
    let node = resolve_active_node(conn);
    info!(
        "kernel.audio: play {} bytes on '{}' (source={:?})",
        audio.len(),
        node.hostname,
        node.source
    );
    if node.is_local() {
        play_local(audio).await;
    } else {
        play_remote(audio, &node.base_url()).await;
    }
}

/// POST WAV bytes to `/api/kernel/play` on a remote mesh node.
pub async fn play_remote(audio: &[u8], base_url: &str) {
    let url = format!("{base_url}/api/kernel/play");
    info!("kernel.audio: POST {} bytes to {url}", audio.len());
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("kernel.audio: reqwest build error: {e}");
            return;
        }
    };
    match client
        .post(&url)
        .header("content-type", "audio/wav")
        .body(audio.to_vec())
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            info!("kernel.audio: remote play accepted status={}", resp.status())
        }
        Ok(resp) => warn!("kernel.audio: remote play rejected status={}", resp.status()),
        Err(e) => warn!("kernel.audio: remote play HTTP error: {e}"),
    }
}
