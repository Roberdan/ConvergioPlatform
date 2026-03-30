use super::{db_path_from_env, resolve_interval_secs};

use std::sync::{Mutex, OnceLock};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
fn env_lock() -> &'static Mutex<()> {
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn test_db_path_from_env_fallback_to_home() {
    let _guard = env_lock().lock().expect("env lock");
    std::env::remove_var("DASHBOARD_DB");
    let path = db_path_from_env();
    assert!(
        path.to_str().unwrap().ends_with(".claude/data/dashboard.db"),
        "fallback must resolve to ~/.claude/data/dashboard.db, got: {}",
        path.display()
    );
}
