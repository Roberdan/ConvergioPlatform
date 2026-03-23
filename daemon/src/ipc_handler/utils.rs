use std::path::PathBuf;

pub fn default_db_path() -> PathBuf {
    // BUG-2 fix: respect DASHBOARD_DB env var, fall back to ~/.claude/data/dashboard.db
    if let Ok(db) = std::env::var("DASHBOARD_DB") {
        if !db.is_empty() {
            return PathBuf::from(db);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude/data/dashboard.db")
}

pub fn default_peers_conf() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude/config/peers.conf")
}
