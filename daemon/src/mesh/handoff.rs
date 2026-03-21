// Handoff: plan delegation between mesh nodes.
// Routing helpers and locking logic are in sibling files.

#[path = "handoff_routing.rs"]
mod handoff_routing;
#[path = "handoff_locking.rs"]
mod handoff_locking;

pub use handoff_routing::{
    check_stale_host, detect_sync_source, parse_peers_conf, resolve_cli_command, PeerConfig,
    StaleHostStatus, SyncSourceInfo,
};
pub use handoff_locking::{acquire_lock, merge_plan_status, release_lock};

#[path = "handoff_ssh.rs"]
mod handoff_ssh;
pub use handoff_ssh::{pull_db_from_peer, SshClient};

#[cfg(test)]
#[path = "handoff_tests.rs"]
mod handoff_tests;
