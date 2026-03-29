// Ali Orchestrator — event-driven plan execution coordinator.
// Listens on #orchestration IPC channel and delegates work to mesh peers.

pub mod actions;
pub mod delegation_core;
mod executor;
pub mod handlers;
mod reactor;
pub mod reaper;

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
    let reaper_db = db_path.clone();
    tokio::spawn(async move {
        tracing::info!("ali-orchestrator: starting");
        if let Err(e) = engine.channel_create(CHANNEL, Some("Plan orchestration events"), ALI_AGENT) {
            tracing::warn!("ali-orchestrator: channel create failed: {e}");
        }
        if let Err(e) = engine.register(ALI_AGENT, "orchestrator", None, &IpcEngine::hostname(), None) {
            tracing::warn!("ali-orchestrator: register failed: {e}");
        }
        reactor::run(engine, db_path).await;
    });
    reaper::spawn_reaper(reaper_db);
}
