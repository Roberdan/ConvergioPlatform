// Ali Orchestrator — event-driven plan execution coordinator.
// Listens on #orchestration IPC channel and delegates work to mesh peers.

pub mod actions;
pub mod approval;
pub mod auto_rollback;
pub mod delegation_core;
pub mod delegation_pipeline;
mod delegation_pipeline_steps;
mod executor;
pub mod goal_decomposer;
pub mod handlers;
pub mod nightly;
pub mod policy;
pub mod reaper;
mod reactor;
pub mod rollback;
pub mod sandbox;
pub mod validator_service;
pub mod worktree_settings;

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
    let db_path_for_validator = db_path.clone();
    tokio::spawn(async move {
        tracing::info!("ali-orchestrator: starting");
        if let Err(e) = engine.channel_create(CHANNEL, Some("Plan orchestration events"), ALI_AGENT) {
            tracing::warn!("ali-orchestrator: channel create failed: {e}");
        }
        if let Err(e) = engine.register(ALI_AGENT, "orchestrator", None, &IpcEngine::hostname(), None, None) {
            tracing::warn!("ali-orchestrator: register failed: {e}");
        }
        reactor::run(engine, db_path).await;
    });
    reaper::spawn_reaper(reaper_db);
    validator_service::spawn_validator_loop(db_path_for_validator);
}
