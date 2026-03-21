#[cfg(test)]
#[path = "../daemon_auth_config_tests.rs"]
mod daemon_auth_config_tests;
#[path = "../daemon_sync.rs"]
mod daemon_sync;
#[cfg(test)]
#[path = "../daemon_tests/mod.rs"]
mod daemon_tests;

mod events;
mod net_utils;
mod peer_loop;
mod service;
mod types;

pub use events::{now_ts, publish_event, relay_agent_activity_changes, relay_ipc_changes};
pub use net_utils::{detect_tailscale_ip, is_ws_brain_request, parse_peers_conf, websocket_key};
pub use service::{handle_ws_client, run_service};
pub use types::{DaemonConfig, DaemonState, InboundConnectionRateLimiter, MeshEvent};

pub(super) const WS_BRAIN_ROUTE: &str = "/ws/brain";
