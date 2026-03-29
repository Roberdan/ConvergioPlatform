use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncRuntimeStatusSnapshot {
    pub healthy: bool,
    pub last_success_at: Option<String>,
    pub last_error: Option<String>,
    pub transport_mode: String,
    pub fallback_policy: String,
}

#[derive(Debug, Clone)]
pub struct SyncRuntimeStatusHolder {
    inner: Arc<RwLock<SyncRuntimeStatusSnapshot>>,
}

impl SyncRuntimeStatusHolder {
    pub fn new_daemon_first() -> Self {
        Self {
            inner: Arc::new(RwLock::new(SyncRuntimeStatusSnapshot {
                healthy: false,
                last_success_at: None,
                last_error: None,
                transport_mode: "daemon-http".to_string(),
                fallback_policy: "manual-rsync-only".to_string(),
            })),
        }
    }

    pub fn snapshot(&self) -> SyncRuntimeStatusSnapshot {
        self.inner.read().expect("sync status poisoned").clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_first_defaults_are_explicit() {
        let status = SyncRuntimeStatusHolder::new_daemon_first().snapshot();
        assert_eq!(status.transport_mode, "daemon-http");
        assert_eq!(status.fallback_policy, "manual-rsync-only");
        assert!(!status.healthy);
        assert!(status.last_success_at.is_none());
        assert!(status.last_error.is_none());
    }
}
