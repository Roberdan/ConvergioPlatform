use super::state::ServerState;

#[test]
fn server_state_initializes_daemon_first_sync_runtime_status() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let db_path = dir.path().join("dashboard.db");
    let state = ServerState::new(db_path, None);
    let status = state.sync_runtime_status().snapshot();
    assert_eq!(status.transport_mode, "daemon-http");
    assert_eq!(status.fallback_policy, "manual-rsync-only");
}
