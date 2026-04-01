// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for the config module.

use super::*;
use std::io::Write as _;
use std::path::PathBuf;

#[test]
fn parse_full_config() {
    let toml_str = r#"
[node]
name = "studio-mac"
role = "coordinator"

[daemon]
port = 9000
auto_update = false
quiet_hours = "22:00-08:00"
timezone = "America/New_York"

[mesh]
transport = "tailscale"
discovery = "static"
peers = ["10.0.0.1:8420", "10.0.0.2:8420"]

[mesh.tailscale]
enabled = true
auth_key = "tskey-abc123"

[inference]
default_model = "claude-opus-4-6"
api_key_env = "MY_KEY"

[kernel]
model = "qwen-7b"
model_path = "/models/qwen"
escalation_model = "claude-opus-4-6"
max_tokens = 4096

[telegram]
enabled = true
token_keychain = "convergio-bot-token"
"#;
    let cfg: ConvergioConfig =
        toml::from_str(toml_str).expect("full config must parse");
    assert_eq!(cfg.node.name, "studio-mac");
    assert_eq!(cfg.node.role, "coordinator");
    assert_eq!(cfg.daemon.port, 9000);
    assert!(!cfg.daemon.auto_update);
    assert_eq!(cfg.mesh.transport, "tailscale");
    assert!(cfg.mesh.tailscale.enabled);
    assert_eq!(cfg.kernel.max_tokens, 4096);
    assert!(cfg.telegram.enabled);
}

#[test]
fn parse_minimal_config() {
    let toml_str = r#"
[node]
role = "worker"
"#;
    let cfg: ConvergioConfig =
        toml::from_str(toml_str).expect("minimal config must parse");
    assert_eq!(cfg.node.role, "worker");
    // everything else is default
    assert_eq!(cfg.daemon.port, 8420);
    assert_eq!(cfg.mesh.transport, "lan");
    assert_eq!(cfg.kernel.model, "none");
}

#[test]
fn empty_string_parses_to_defaults() {
    let cfg: ConvergioConfig =
        toml::from_str("").expect("empty string must parse");
    assert_eq!(cfg.daemon.port, 8420);
    assert_eq!(cfg.node.role, "standalone");
}

#[test]
fn defaults_are_sensible() {
    let cfg = ConvergioConfig::default();
    assert_eq!(cfg.daemon.port, 8420);
    assert_eq!(cfg.node.role, "standalone");
    assert_eq!(cfg.mesh.transport, "lan");
    assert_eq!(cfg.inference.default_model, "claude-sonnet-4-6");
    assert_eq!(cfg.kernel.model, "none");
    assert_eq!(cfg.kernel.max_tokens, 2048);
    assert!(!cfg.telegram.enabled);
    assert!(cfg.daemon.auto_update);
}

#[test]
fn write_default_produces_parseable_toml() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("config.toml");
    write_default_config(&path).expect("write default config");
    let contents =
        std::fs::read_to_string(&path).expect("read written file");
    let cfg: ConvergioConfig =
        toml::from_str(&contents).expect("template must parse");
    assert_eq!(cfg.daemon.port, 8420);
}

#[test]
fn config_path_env_override() {
    let original = std::env::var("CONVERGIO_CONFIG").ok();
    std::env::set_var("CONVERGIO_CONFIG", "/tmp/test-cfg.toml");
    assert_eq!(config_path(), PathBuf::from("/tmp/test-cfg.toml"));
    match original {
        Some(v) => std::env::set_var("CONVERGIO_CONFIG", v),
        None => std::env::remove_var("CONVERGIO_CONFIG"),
    }
}

#[test]
fn load_from_temp_file() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("config.toml");
    let mut f = std::fs::File::create(&path).expect("create file");
    f.write_all(b"[daemon]\nport = 7777\n").expect("write");

    let original = std::env::var("CONVERGIO_CONFIG").ok();
    std::env::set_var("CONVERGIO_CONFIG", path.to_str().unwrap());
    let cfg = load_config();
    assert_eq!(cfg.daemon.port, 7777);
    match original {
        Some(v) => std::env::set_var("CONVERGIO_CONFIG", v),
        None => std::env::remove_var("CONVERGIO_CONFIG"),
    }
}
