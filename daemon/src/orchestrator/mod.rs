// Ali Orchestrator — event-driven plan execution coordinator.
// Listens on #orchestration IPC channel and delegates work to mesh peers.

mod actions;
mod handlers;
mod reactor;

use crate::ipc::IpcEngine;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const ALI_AGENT: &str = "ali-orchestrator";
pub const CHANNEL: &str = "#orchestration";

static ALI_SPAWNED: AtomicBool = AtomicBool::new(false);

/// Spawn the Ali orchestrator as a background tokio task.
/// Safe to call multiple times — only the first call spawns Ali.
pub fn spawn_ali(engine: Arc<IpcEngine>, db_path: PathBuf) {
    if ALI_SPAWNED.swap(true, Ordering::SeqCst) {
        tracing::debug!("ali-orchestrator: already running, skipping duplicate spawn");
        return;
    }
    tokio::spawn(async move {
        tracing::info!("ali-orchestrator: starting");
        let _ = engine.channel_create(CHANNEL, Some("Plan orchestration events"), ALI_AGENT);
        let _ = engine.register(ALI_AGENT, "orchestrator", None, &IpcEngine::hostname(), None);
        reactor::run(engine, db_path).await;
    });
}
