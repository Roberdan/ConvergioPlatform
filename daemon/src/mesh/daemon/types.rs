// Daemon core types: config, state, rate limiter

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub bind_ip: String,
    pub port: u16,
    pub peers_conf_path: PathBuf,
    pub db_path: PathBuf,
    pub crsqlite_path: Option<String>,
    pub local_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshEvent {
    pub kind: String,
    pub node: String,
    pub ts: u64,
    pub payload: Value,
}

#[derive(Clone)]
pub struct DaemonState {
    pub node_id: String,
    pub tx: broadcast::Sender<MeshEvent>,
    pub heartbeats: Arc<RwLock<HashMap<String, u64>>>,
}

pub struct InboundConnectionRateLimiter {
    pub minute_limiter: crate::mesh::observability::RateLimiter,
    pub second_windows: std::sync::Mutex<HashMap<IpAddr, Vec<std::time::Instant>>>,
    pub max_per_second: usize,
}

impl InboundConnectionRateLimiter {
    pub fn new(max_per_second: usize, max_per_minute: usize) -> Self {
        Self {
            minute_limiter: crate::mesh::observability::RateLimiter::new(max_per_minute, 100),
            second_windows: std::sync::Mutex::new(HashMap::new()),
            max_per_second,
        }
    }

    pub fn check(&self, remote: SocketAddr) -> Result<(), String> {
        let ip = remote.ip();
        {
            let mut windows = self.second_windows.lock().unwrap();
            let entry = windows.entry(ip).or_default();
            let cutoff = std::time::Instant::now() - Duration::from_secs(1);
            entry.retain(|ts| *ts > cutoff);
            if entry.len() >= self.max_per_second {
                return Err(format!(
                    "per-second limit ({}/sec) exceeded for {ip}",
                    self.max_per_second
                ));
            }
            entry.push(std::time::Instant::now());
        }
        self.minute_limiter.check_and_record(ip)
    }

    pub fn release(&self, remote: SocketAddr) {
        self.minute_limiter.release(remote.ip());
    }
}
