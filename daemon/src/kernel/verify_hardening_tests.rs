// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for verify_hardening: mutex, SHA cache, and build process reaper.

#[cfg(test)]
mod tests {
    use crate::kernel::verify_hardening::{
        reap_build_processes, EvidenceCache, EVIDENCE_MUTEX,
    };

    // --- Mutex serialization ---

    #[test]
    fn evidence_mutex_exists_and_is_lockable() {
        let guard = EVIDENCE_MUTEX.lock().expect("mutex lock");
        drop(guard);
    }

    #[test]
    fn evidence_mutex_serializes_concurrent_access() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..4 {
            let c = Arc::clone(&counter);
            handles.push(std::thread::spawn(move || {
                let _guard = EVIDENCE_MUTEX.lock().expect("lock");
                let prev = c.load(Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(5));
                c.store(prev + 1, Ordering::SeqCst);
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    // --- SHA-based evidence cache ---

    #[test]
    fn evidence_cache_returns_none_initially() {
        let cache = EvidenceCache::new();
        assert!(cache.get("abc123").is_none());
    }

    #[test]
    fn evidence_cache_hit_within_ttl() {
        let cache = EvidenceCache::new();
        cache.store("sha_abc", true);
        assert_eq!(cache.get("sha_abc"), Some(true));
    }

    #[test]
    fn evidence_cache_miss_on_different_sha() {
        let cache = EvidenceCache::new();
        cache.store("sha_old", true);
        assert!(cache.get("sha_new").is_none());
    }

    #[test]
    fn evidence_cache_expires_after_ttl() {
        let cache = EvidenceCache::new();
        cache.store("sha_x", true);
        cache.force_expire_for_test();
        assert!(cache.get("sha_x").is_none());
    }

    #[test]
    fn evidence_cache_stores_failure() {
        let cache = EvidenceCache::new();
        cache.store("sha_fail", false);
        assert_eq!(cache.get("sha_fail"), Some(false));
    }

    // --- Build process reaper ---

    #[test]
    fn reap_build_processes_runs_without_panic() {
        reap_build_processes();
    }
}
