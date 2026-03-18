use super::{daemon_sync, validate_config, DaemonConfig};
use std::path::PathBuf;

fn test_config(peers_conf_path: PathBuf) -> DaemonConfig {
    DaemonConfig {
        bind_ip: "127.0.0.1".to_string(),
        port: 9420,
        peers_conf_path,
        db_path: PathBuf::from("/tmp"),
        crsqlite_path: None,
        local_only: false,
    }
}

#[test]
fn validate_config_rejects_missing_shared_secret() {
    let peers_conf = std::env::temp_dir().join("mesh_missing_shared_secret.conf");
    std::fs::write(&peers_conf, "[peer1]\ntailscale_ip=100.1.2.3\n").expect("write peers conf");
    let err = validate_config(&test_config(peers_conf.clone())).expect_err("must reject config");
    assert!(
        err.contains("shared_secret"),
        "error should mention shared_secret: {err}"
    );
    let _ = std::fs::remove_file(peers_conf);
}

#[test]
fn auth_secret_loader_rejects_empty_secret() {
    let peers_conf = std::env::temp_dir().join("mesh_empty_shared_secret.conf");
    std::fs::write(&peers_conf, "[mesh]\nshared_secret = \n").expect("write peers conf");
    let err = daemon_sync::load_required_shared_secret(&peers_conf)
        .expect_err("must reject empty secret");
    assert!(
        err.contains("shared_secret"),
        "error should mention shared_secret: {err}"
    );
    let _ = std::fs::remove_file(peers_conf);
}
