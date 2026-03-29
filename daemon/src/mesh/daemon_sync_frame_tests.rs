use super::frame::resolve_peer_name;

#[test]
fn resolve_peer_name_by_tailscale_ip() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "[mesh]\nshared_secret=s\n\n[m5max]\ntailscale_ip=100.89.245.79\n",
    )
    .unwrap();
    let name = resolve_peer_name(tmp.path(), "100.89.245.79:9420");
    assert_eq!(name, "m5max");
}

#[test]
fn resolve_peer_name_falls_back_to_node_id() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "[mesh]\nshared_secret=s\n\n[m5max]\ntailscale_ip=100.89.245.79\n",
    )
    .unwrap();
    let name = resolve_peer_name(tmp.path(), "unknown-node:9420");
    assert_eq!(name, "unknown-node:9420");
}

#[test]
fn resolve_peer_name_handles_missing_file() {
    let name = resolve_peer_name(
        std::path::Path::new("/tmp/nonexistent_peers_11223344.conf"),
        "node-x:9420",
    );
    assert_eq!(name, "node-x:9420");
}

#[test]
fn resolve_peer_name_strips_port_for_ip_lookup() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "[mesh]\nshared_secret=s\n\n[worker]\ntailscale_ip=100.64.0.2\n",
    )
    .unwrap();
    // node = "100.64.0.2:9420" → ip = "100.64.0.2" → matches worker
    let name = resolve_peer_name(tmp.path(), "100.64.0.2:9420");
    assert_eq!(name, "worker");
}
