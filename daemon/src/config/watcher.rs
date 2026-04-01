// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Filesystem watcher for config hot-reload with debounce and diff.

use super::ConvergioConfig;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Fields that can be updated at runtime without a restart.
const HOT_RELOADABLE: &[&str] = &[
    "daemon.quiet_hours",
    "daemon.timezone",
    "daemon.auto_update",
    "inference.default_model",
    "kernel.max_tokens",
    "mesh.peers",
    "telegram.enabled",
];

/// Fields that require a restart — validated in tests.
#[cfg(test)]
const RESTART_REQUIRED: &[&str] = &[
    "node.role",
    "daemon.port",
    "mesh.transport",
];

// -------------------------------------------------------------------------
// Diff
// -------------------------------------------------------------------------

/// A single changed field with its new value (as display string).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigChange {
    pub field: String,
    pub reloadable: bool,
}

/// Compare two configs and return the list of fields that differ.
pub fn diff_configs(
    old: &ConvergioConfig,
    new: &ConvergioConfig,
) -> Vec<ConfigChange> {
    let mut changes = Vec::new();
    macro_rules! cmp {
        ($field:expr, $a:expr, $b:expr) => {
            if $a != $b {
                let name: &str = $field;
                let reloadable = HOT_RELOADABLE.contains(&name);
                changes.push(ConfigChange {
                    field: name.to_string(),
                    reloadable,
                });
            }
        };
    }
    // node
    cmp!("node.role", old.node.role, new.node.role);
    cmp!("node.name", old.node.name, new.node.name);
    // daemon
    cmp!("daemon.port", old.daemon.port, new.daemon.port);
    cmp!("daemon.quiet_hours", old.daemon.quiet_hours, new.daemon.quiet_hours);
    cmp!("daemon.timezone", old.daemon.timezone, new.daemon.timezone);
    cmp!("daemon.auto_update", old.daemon.auto_update, new.daemon.auto_update);
    // mesh
    cmp!("mesh.transport", old.mesh.transport, new.mesh.transport);
    cmp!("mesh.peers", old.mesh.peers, new.mesh.peers);
    // inference
    cmp!(
        "inference.default_model",
        old.inference.default_model,
        new.inference.default_model
    );
    // kernel
    cmp!("kernel.max_tokens", old.kernel.max_tokens, new.kernel.max_tokens);
    // telegram
    cmp!("telegram.enabled", old.telegram.enabled, new.telegram.enabled);
    changes
}

// -------------------------------------------------------------------------
// Apply
// -------------------------------------------------------------------------

/// Apply only hot-reloadable changes from `new` into `current`.
fn apply_reloadable(current: &mut ConvergioConfig, new: &ConvergioConfig) {
    current.daemon.quiet_hours = new.daemon.quiet_hours.clone();
    current.daemon.timezone = new.daemon.timezone.clone();
    current.daemon.auto_update = new.daemon.auto_update;
    current.inference.default_model = new.inference.default_model.clone();
    current.kernel.max_tokens = new.kernel.max_tokens;
    current.mesh.peers = new.mesh.peers.clone();
    current.telegram.enabled = new.telegram.enabled;
}

// -------------------------------------------------------------------------
// Debounce helper
// -------------------------------------------------------------------------

const DEBOUNCE_MS: u128 = 500;

/// Returns true if enough time has elapsed since `last`.
fn should_reload(last: &Instant) -> bool {
    last.elapsed().as_millis() >= DEBOUNCE_MS
}

// -------------------------------------------------------------------------
// Watcher
// -------------------------------------------------------------------------

/// Spawn a background thread that watches the config file for changes.
/// On modify, debounces 500 ms, re-parses, diffs, and applies reloadable
/// fields. Parse errors are logged and the current config is kept.
pub fn spawn_config_watcher(
    config: Arc<RwLock<ConvergioConfig>>,
) -> Result<(), String> {
    let path = super::config_path();
    let watch_dir = path
        .parent()
        .ok_or_else(|| "config path has no parent directory".to_string())?
        .to_path_buf();
    if !watch_dir.exists() {
        return Err(format!(
            "config directory does not exist: {}",
            watch_dir.display()
        ));
    }
    let file_name = path
        .file_name()
        .map(|f| f.to_os_string())
        .unwrap_or_default();

    let cfg = Arc::clone(&config);
    let last_reload = Arc::new(std::sync::Mutex::new(Instant::now()));

    let lr = Arc::clone(&last_reload);
    let fname = file_name.clone();
    let cfg_path = path.clone();
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            let event = match res {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("[config] watcher error: {e}");
                    return;
                }
            };
            if !matches!(event.kind, EventKind::Modify(_)) {
                return;
            }
            // Only react to the config file itself.
            let is_target = event
                .paths
                .iter()
                .any(|p| p.file_name().map(|f| f == fname).unwrap_or(false));
            if !is_target {
                return;
            }
            // Debounce.
            let mut last = lr.lock().unwrap_or_else(|e| e.into_inner());
            if !should_reload(&last) {
                return;
            }
            *last = Instant::now();
            drop(last);
            reload_config(&cfg, &cfg_path);
        },
        Config::default(),
    )
    .map_err(|e| format!("watcher init failed: {e}"))?;

    watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .map_err(|e| format!("watch failed: {e}"))?;

    // Leak the watcher so it lives for the process lifetime.
    std::mem::forget(watcher);
    tracing::info!(
        "[config] Watching {} for changes",
        path.display()
    );
    Ok(())
}

/// Re-read, diff, and apply reloadable config changes.
fn reload_config(
    config: &Arc<RwLock<ConvergioConfig>>,
    path: &std::path::Path,
) {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                "[config] Failed to read {}: {e} — keeping current config",
                path.display()
            );
            return;
        }
    };
    let new_cfg = match toml::from_str::<ConvergioConfig>(&contents) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                "[config] Parse error in {}: {e} — keeping current config",
                path.display()
            );
            return;
        }
    };
    let mut guard = match config.write() {
        Ok(g) => g,
        Err(e) => {
            tracing::warn!("[config] RwLock poisoned: {e}");
            return;
        }
    };
    let changes = diff_configs(&guard, &new_cfg);
    if changes.is_empty() {
        return;
    }
    for change in &changes {
        if change.reloadable {
            tracing::info!("[config] Reloaded: {} changed", change.field);
        } else {
            tracing::warn!(
                "[config] {} changed but requires restart to take effect",
                change.field
            );
        }
    }
    apply_reloadable(&mut guard, &new_cfg);
}

#[cfg(test)]
#[path = "watcher_tests.rs"]
mod tests;
