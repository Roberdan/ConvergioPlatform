// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for config watcher: diff detection, debounce, parse resilience.

use super::*;
use crate::config::ConvergioConfig;
use std::sync::{Arc, RwLock};
use std::time::Instant;

// -------------------------------------------------------------------------
// Diff detection
// -------------------------------------------------------------------------

#[test]
fn diff_identical_configs_is_empty() {
    let a = ConvergioConfig::default();
    let b = ConvergioConfig::default();
    assert!(diff_configs(&a, &b).is_empty());
}

#[test]
fn diff_detects_reloadable_field() {
    let old = ConvergioConfig::default();
    let mut new = old.clone();
    new.daemon.auto_update = !old.daemon.auto_update;
    let changes = diff_configs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].field, "daemon.auto_update");
    assert!(changes[0].reloadable);
}

#[test]
fn diff_detects_non_reloadable_field() {
    let old = ConvergioConfig::default();
    let mut new = old.clone();
    new.daemon.port = 9999;
    let changes = diff_configs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].field, "daemon.port");
    assert!(!changes[0].reloadable);
}

#[test]
fn diff_detects_multiple_changes() {
    let old = ConvergioConfig::default();
    let mut new = old.clone();
    new.kernel.max_tokens = 8192;
    new.telegram.enabled = true;
    new.node.role = "coordinator".to_string();
    let changes = diff_configs(&old, &new);
    assert_eq!(changes.len(), 3);
    let fields: Vec<&str> =
        changes.iter().map(|c| c.field.as_str()).collect();
    assert!(fields.contains(&"kernel.max_tokens"));
    assert!(fields.contains(&"telegram.enabled"));
    assert!(fields.contains(&"node.role"));
}

#[test]
fn diff_quiet_hours_change() {
    let old = ConvergioConfig::default();
    let mut new = old.clone();
    new.daemon.quiet_hours = Some("22:00-06:00".to_string());
    let changes = diff_configs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].field, "daemon.quiet_hours");
    assert!(changes[0].reloadable);
}

#[test]
fn diff_peers_change() {
    let old = ConvergioConfig::default();
    let mut new = old.clone();
    new.mesh.peers = vec!["10.0.0.1:8420".to_string()];
    let changes = diff_configs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].field, "mesh.peers");
    assert!(changes[0].reloadable);
}

#[test]
fn diff_inference_model_change() {
    let old = ConvergioConfig::default();
    let mut new = old.clone();
    new.inference.default_model = "claude-opus-4-6".to_string();
    let changes = diff_configs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].field, "inference.default_model");
    assert!(changes[0].reloadable);
}

// -------------------------------------------------------------------------
// Debounce
// -------------------------------------------------------------------------

#[test]
fn debounce_blocks_rapid_reloads() {
    let now = Instant::now();
    // Immediately after creation, should not reload.
    assert!(!should_reload(&now));
}

#[test]
fn debounce_allows_after_threshold() {
    // Create an instant in the past.
    let past = Instant::now() - std::time::Duration::from_millis(600);
    assert!(should_reload(&past));
}

// -------------------------------------------------------------------------
// Parse error resilience
// -------------------------------------------------------------------------

#[test]
fn parse_error_keeps_current_config() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("config.toml");
    // Write valid config first.
    std::fs::write(&path, "[daemon]\nport = 7777\n").expect("write");
    let cfg: ConvergioConfig =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let shared = Arc::new(RwLock::new(cfg));
    // Now write invalid TOML.
    std::fs::write(&path, "[[[[invalid toml garbage").expect("write");
    // Trigger reload — should keep old config.
    reload_config(&shared, &path);
    let guard = shared.read().unwrap();
    assert_eq!(guard.daemon.port, 7777);
}

#[test]
fn reload_applies_reloadable_fields_only() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("config.toml");
    // Start with defaults.
    let shared = Arc::new(RwLock::new(ConvergioConfig::default()));
    assert_eq!(shared.read().unwrap().daemon.port, 8420);
    assert!(shared.read().unwrap().daemon.auto_update);
    // Write config that changes both reloadable and non-reloadable.
    std::fs::write(
        &path,
        "[daemon]\nport = 9999\nauto_update = false\n",
    )
    .expect("write");
    reload_config(&shared, &path);
    let guard = shared.read().unwrap();
    // Reloadable field applied.
    assert!(!guard.daemon.auto_update);
    // Non-reloadable field NOT applied.
    assert_eq!(guard.daemon.port, 8420);
}

#[test]
fn reload_missing_file_keeps_config() {
    let shared = Arc::new(RwLock::new(ConvergioConfig::default()));
    let path = std::path::Path::new("/tmp/nonexistent-convergio-cfg.toml");
    reload_config(&shared, path);
    // Config unchanged.
    assert_eq!(shared.read().unwrap().daemon.port, 8420);
}

// -------------------------------------------------------------------------
// Restart-required fields list
// -------------------------------------------------------------------------

#[test]
fn restart_required_fields_are_not_reloadable() {
    for field in RESTART_REQUIRED {
        assert!(
            !HOT_RELOADABLE.contains(field),
            "{field} should not be in HOT_RELOADABLE"
        );
    }
}
