// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Evidence gate hardening: mutex serialization, SHA-based cache, build reaper.
//
// WHY: The evidence gate spawns `cargo test` on every task transition.
// Without guards this causes 482 threads, 50% CPU, 500MB+ RAM per run,
// with no timeout and no concurrency limit.

use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Static mutex: only 1 evidence check at a time (prevents resource exhaustion)
// ---------------------------------------------------------------------------

pub static EVIDENCE_MUTEX: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// SHA-based evidence cache — skip re-check if git HEAD unchanged (5min TTL)
// ---------------------------------------------------------------------------

const CACHE_TTL: Duration = Duration::from_secs(300);

pub struct EvidenceCache {
    inner: Mutex<Option<CacheEntry>>,
}

struct CacheEntry {
    sha: String,
    passed: bool,
    at: Instant,
}

impl EvidenceCache {
    pub fn new() -> Self {
        Self { inner: Mutex::new(None) }
    }

    pub fn get(&self, sha: &str) -> Option<bool> {
        let guard = match self.inner.lock() {
            Ok(g) => g,
            Err(e) => {
                tracing::warn!("verify_hardening: evidence cache lock poisoned: {e}");
                return None;
            }
        };
        let entry = guard.as_ref()?;
        if entry.sha == sha && entry.at.elapsed() < CACHE_TTL {
            Some(entry.passed)
        } else {
            None
        }
    }

    pub fn store(&self, sha: &str, passed: bool) {
        if let Ok(mut guard) = self.inner.lock() {
            *guard = Some(CacheEntry {
                sha: sha.to_string(),
                passed,
                at: Instant::now(),
            });
        }
    }

    /// Test helper: force cache entry to appear expired.
    #[cfg(test)]
    pub fn force_expire_for_test(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            if let Some(entry) = guard.as_mut() {
                entry.at = Instant::now() - CACHE_TTL - Duration::from_secs(1);
            }
        }
    }
}

/// Global evidence cache instance.
pub static EVIDENCE_CACHE: std::sync::LazyLock<EvidenceCache> =
    std::sync::LazyLock::new(EvidenceCache::new);

/// Read git HEAD SHA for a worktree (or cwd).
pub fn git_head_sha(worktree: Option<&str>) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "HEAD"]);
    if let Some(wt) = worktree {
        cmd.current_dir(wt);
    }
    match cmd.output() {
        Ok(o) if o.status.success() => Some(String::from_utf8_lossy(&o.stdout).trim().to_string()),
        Ok(_) => None,
        Err(e) => {
            tracing::warn!("verify_hardening: git rev-parse failed: {e}");
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Reaper: kill orphaned cargo/rustc spawned by evidence gate
// ---------------------------------------------------------------------------

/// Kill any cargo/rustc processes that are children of this daemon.
/// Called on graceful shutdown to prevent zombie build processes.
pub fn reap_build_processes() {
    #[cfg(unix)]
    {
        let our_pid = std::process::id().to_string();
        for proc_name in &["cargo", "rustc"] {
            let output = Command::new("pgrep")
                .args(["-P", &our_pid, proc_name])
                .output();
            if let Ok(out) = output {
                let pids = String::from_utf8_lossy(&out.stdout);
                for pid_str in pids.split_whitespace() {
                    if let Ok(pid) = pid_str.parse::<i32>() {
                        unsafe { libc::kill(pid, libc::SIGTERM); }
                        tracing::info!(
                            pid, proc_name, "evidence reaper: sent SIGTERM"
                        );
                    }
                }
            }
        }
    }
}
