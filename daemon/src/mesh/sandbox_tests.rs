use super::*;

#[test]
fn sandbox_generates_hardened_docker_args() {
    let cfg = SandboxConfig::default();
    let args = cfg.to_docker_args();
    assert!(args.contains(&"--security-opt=no-new-privileges".into()));
    assert!(args.contains(&"--cap-drop=ALL".into()));
    assert!(args.contains(&"--pids-limit=256".into()));
    assert!(args.contains(&"--cpus=2".into()));
}

#[test]
fn night_mode_crosses_midnight() {
    let cfg = NightModeConfig::default(); // 22-06
    assert!(cfg.is_active_at_hour(23));
    assert!(cfg.is_active_at_hour(3));
    assert!(!cfg.is_active_at_hour(12));
    assert!(!cfg.is_active_at_hour(8));
}

#[test]
fn night_mode_disabled() {
    let cfg = NightModeConfig {
        enabled: false,
        ..Default::default()
    };
    assert!(!cfg.is_active_at_hour(23));
}

// ── Additional tests ─────────────────────────────────────────────────────────

#[test]
fn sandbox_default_values() {
    let cfg = SandboxConfig::default();
    assert_eq!(cfg.image, "convergio-mesh:latest");
    assert!((cfg.cpu_limit - 2.0).abs() < 0.01);
    assert_eq!(cfg.memory_limit_mb, 4096);
    assert_eq!(cfg.timeout_secs, 3600);
    assert_eq!(cfg.volumes.len(), 1);
    assert!(!cfg.volumes[0].read_only);
}

#[test]
fn sandbox_network_none_arg() {
    let cfg = SandboxConfig {
        network: SandboxNetwork::None,
        ..Default::default()
    };
    let args = cfg.to_docker_args();
    assert!(args.contains(&"--network=none".into()));
}

#[test]
fn sandbox_network_host_arg() {
    let cfg = SandboxConfig {
        network: SandboxNetwork::Host,
        ..Default::default()
    };
    let args = cfg.to_docker_args();
    assert!(args.contains(&"--network=host".into()));
}

#[test]
fn sandbox_env_vars_included() {
    let mut env = std::collections::HashMap::new();
    env.insert("DASHBOARD_DB".to_string(), "/data/db".to_string());
    env.insert("NODE_ROLE".to_string(), "worker".to_string());
    let cfg = SandboxConfig {
        env_vars: env,
        ..Default::default()
    };
    let args = cfg.to_docker_args();
    assert!(args.iter().any(|a| a.starts_with("-eDASHBOARD_DB=")));
    assert!(args.iter().any(|a| a.starts_with("-eNODE_ROLE=")));
}

#[test]
fn sandbox_volume_read_only_flag() {
    let cfg = SandboxConfig {
        volumes: vec![VolumeMount {
            host_path: "/host/data".into(),
            container_path: "/container/data".into(),
            read_only: true,
        }],
        ..Default::default()
    };
    let args = cfg.to_docker_args();
    assert!(args.iter().any(|a| a.contains(":ro")));
}

#[test]
fn sandbox_memory_limit_arg() {
    let cfg = SandboxConfig {
        memory_limit_mb: 8192,
        ..Default::default()
    };
    let args = cfg.to_docker_args();
    assert!(args.contains(&"--memory=8192m".into()));
}

#[test]
fn sandbox_custom_image() {
    let cfg = SandboxConfig {
        image: "custom-mesh:v2".into(),
        ..Default::default()
    };
    let args = cfg.to_docker_args();
    assert_eq!(args.last().unwrap(), "custom-mesh:v2");
}

#[test]
fn sandbox_serialization_roundtrip() {
    let cfg = SandboxConfig::default();
    let json = serde_json::to_string(&cfg).unwrap();
    let back: SandboxConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.image, cfg.image);
    assert_eq!(back.timeout_secs, cfg.timeout_secs);
}

#[test]
fn night_mode_same_day_window() {
    let cfg = NightModeConfig {
        enabled: true,
        start_hour: 9,
        end_hour: 17,
        timezone: "CET".into(),
        max_concurrent_tasks: 3,
        allowed_models: vec![],
    };
    assert!(!cfg.is_active_at_hour(8));
    assert!(cfg.is_active_at_hour(9));
    assert!(cfg.is_active_at_hour(12));
    assert!(!cfg.is_active_at_hour(17));
    assert!(!cfg.is_active_at_hour(22));
}

#[test]
fn night_mode_boundary_hours() {
    let cfg = NightModeConfig::default(); // 22-06
    assert!(cfg.is_active_at_hour(22)); // start hour included
    assert!(!cfg.is_active_at_hour(6)); // end hour excluded
    assert!(cfg.is_active_at_hour(0)); // midnight
    assert!(cfg.is_active_at_hour(5));
}

#[test]
fn night_mode_serialization_roundtrip() {
    let cfg = NightModeConfig::default();
    let json = serde_json::to_string(&cfg).unwrap();
    let back: NightModeConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.start_hour, 22);
    assert_eq!(back.end_hour, 6);
    assert!(back.enabled);
}
