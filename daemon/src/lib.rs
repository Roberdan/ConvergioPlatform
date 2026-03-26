pub mod background;
pub mod background_sync;
pub mod channels;
pub mod checklist;
pub mod db;
pub mod digest;
pub mod inference;
pub mod errors;
pub mod hooks;
pub mod ipc;
pub mod lock;
pub mod mesh;
pub mod message_error;
pub mod orchestrator;
pub mod platform_paths;
pub mod resilience;
pub mod server;
pub mod tui;
pub mod validation;
pub mod workspace;

/// Resolve the dashboard DB path from the DASHBOARD_DB env var, falling back
/// to ~/.claude/data/dashboard.db. Used wherever the daemon needs to open the
/// real persistent DB (e.g. background sync loop).
pub fn db_path_from_env() -> std::path::PathBuf {
    if let Ok(db) = std::env::var("DASHBOARD_DB") {
        if !db.is_empty() {
            return std::path::PathBuf::from(db);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".claude/data/dashboard.db")
}
