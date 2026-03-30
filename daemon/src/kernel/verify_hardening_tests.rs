// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for verify_hardening: mutex, SHA cache, and build process reaper.

#[cfg(test)]
mod tests {
    use crate::kernel::verify_hardening::{
        evidence_cache_key, reap_build_processes, EvidenceCache, EVIDENCE_MUTEX,
    };
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    fn git(args: &[&str], dir: &std::path::Path) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("run git");
        assert!(status.success(), "git {:?} failed", args);
    }

    fn init_git_repo() -> tempfile::TempDir {
        let dir = tempdir().expect("tempdir");
        git(&["init"], dir.path());
        git(&["config", "user.email", "tests@example.com"], dir.path());
        git(&["config", "user.name", "Tests"], dir.path());
        fs::write(dir.path().join("tracked.txt"), "first\n").expect("write tracked");
        git(&["add", "tracked.txt"], dir.path());
        git(&["commit", "-m", "first"], dir.path());
        dir
    }

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

    // --- Fingerprint-based evidence cache ---

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
    fn evidence_cache_miss_on_different_key() {
        let cache = EvidenceCache::new();
        cache.store("fp_old", true);
        assert!(cache.get("fp_new").is_none());
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
        cache.store("fp_fail", false);
        assert_eq!(cache.get("fp_fail"), Some(false));
    }

    #[test]
    fn evidence_cache_clear_drops_entry() {
        let cache = EvidenceCache::new();
        cache.store("fp_clear", true);
        cache.clear();
        assert!(cache.get("fp_clear").is_none());
    }

    #[test]
    fn evidence_cache_key_changes_with_output_files() {
        let key_a = evidence_cache_key(Some(env!("CARGO_MANIFEST_DIR")), &["a.txt"])
            .expect("key a");
        let key_b = evidence_cache_key(Some(env!("CARGO_MANIFEST_DIR")), &["b.txt"])
            .expect("key b");
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn evidence_cache_key_changes_when_git_status_changes() {
        let dir = init_git_repo();
        let clean_key =
            evidence_cache_key(dir.path().to_str(), &[]).expect("clean key");

        fs::write(dir.path().join("tracked.txt"), "dirty\n").expect("mutate tracked");
        let dirty_key =
            evidence_cache_key(dir.path().to_str(), &[]).expect("dirty key");

        assert_ne!(clean_key, dirty_key);
    }

    #[test]
    fn evidence_cache_key_changes_when_head_changes() {
        let dir = init_git_repo();
        let first_key =
            evidence_cache_key(dir.path().to_str(), &[]).expect("first key");

        fs::write(dir.path().join("tracked.txt"), "second\n").expect("rewrite tracked");
        git(&["add", "tracked.txt"], dir.path());
        git(&["commit", "-m", "second"], dir.path());
        let second_key =
            evidence_cache_key(dir.path().to_str(), &[]).expect("second key");

        assert_ne!(first_key, second_key);
    }

    // --- Build process reaper ---

    #[test]
    fn reap_build_processes_runs_without_panic() {
        reap_build_processes();
    }
}
