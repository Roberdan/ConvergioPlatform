use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock, RwLock};

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

fn shared_inner() -> Arc<RwLock<SyncRuntimeStatusSnapshot>> {
    static SHARED: OnceLock<Arc<RwLock<SyncRuntimeStatusSnapshot>>> = OnceLock::new();
    SHARED
        .get_or_init(|| {
            Arc::new(RwLock::new(SyncRuntimeStatusSnapshot {
                healthy: false,
                last_success_at: None,
                last_error: None,
                transport_mode: "daemon-http".to_string(),
                fallback_policy: "manual-rsync-only".to_string(),
            }))
        })
        .clone()
}

impl SyncRuntimeStatusHolder {
    pub fn new_daemon_first() -> Self {
        Self {
            inner: shared_inner(),
        }
    }

    pub fn snapshot(&self) -> SyncRuntimeStatusSnapshot {
        self.inner.read().expect("sync status poisoned").clone()
    }

    pub fn mark_success(&self, timestamp: impl Into<String>) {
        let mut status = self.inner.write().expect("sync status poisoned");
        status.healthy = true;
        status.last_success_at = Some(timestamp.into());
        status.last_error = None;
    }

    pub fn mark_error(&self, error: impl Into<String>) {
        let mut status = self.inner.write().expect("sync status poisoned");
        status.healthy = false;
        status.last_error = Some(error.into());
    }

    pub fn reset(&self) {
        let mut status = self.inner.write().expect("sync status poisoned");
        status.healthy = false;
        status.last_success_at = None;
        status.last_error = None;
        status.transport_mode = "daemon-http".to_string();
        status.fallback_policy = "manual-rsync-only".to_string();
    }
}

/// Shared lock for tests that mutate the global SyncRuntimeStatusHolder.
#[cfg(test)]
pub(crate) fn global_status_test_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_first_defaults_are_explicit() {
        let _guard = global_status_test_lock().lock().expect("test lock");
        let holder = SyncRuntimeStatusHolder::new_daemon_first();
        holder.reset();
        let status = holder.snapshot();
        assert_eq!(status.transport_mode, "daemon-http");
        assert_eq!(status.fallback_policy, "manual-rsync-only");
        assert!(!status.healthy);
        assert!(status.last_success_at.is_none());
        assert!(status.last_error.is_none());
    }

    #[test]
    fn holder_is_shared_across_clones() {
        let _guard = global_status_test_lock().lock().expect("test lock");
        let holder_a = SyncRuntimeStatusHolder::new_daemon_first();
        holder_a.reset();
        holder_a.mark_success("2026-03-29T20:00:00Z");

        let holder_b = SyncRuntimeStatusHolder::new_daemon_first();
        let status = holder_b.snapshot();
        assert!(status.healthy);
        assert_eq!(status.last_success_at.as_deref(), Some("2026-03-29T20:00:00Z"));
        assert!(status.last_error.is_none());
    }
}
