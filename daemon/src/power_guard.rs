//! Platform-specific sleep prevention while agents are active.
//! Spawns an OS sleep inhibitor when the first agent registers,
//! kills it when the last deregisters.

use std::process::{Child, Command};
use std::sync::{Mutex, OnceLock};

use serde::Serialize;

static GUARD: OnceLock<Mutex<PowerGuardInner>> = OnceLock::new();

fn guard() -> &'static Mutex<PowerGuardInner> {
    GUARD.get_or_init(|| Mutex::new(PowerGuardInner::new()))
}

struct PowerGuardInner {
    process: Option<Child>,
    agent_count: u32,
}

impl PowerGuardInner {
    fn new() -> Self {
        Self { process: None, agent_count: 0 }
    }
}

impl Drop for PowerGuardInner {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
            let _ = child.wait();
            tracing::info!("power_guard: inhibitor killed on drop");
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PowerGuardStatus {
    pub active: bool,
    pub agent_count: u32,
    pub platform: &'static str,
}

pub struct PowerGuard;

impl PowerGuard {
    /// Call when an agent registers.
    pub fn acquire() {
        let mut g = guard().lock().expect("power_guard lock poisoned");
        g.agent_count += 1;
        if g.agent_count == 1 && g.process.is_none() {
            g.process = spawn_inhibitor();
            if g.process.is_some() {
                tracing::info!(
                    "power_guard: sleep inhibitor started (agents active)"
                );
            }
        }
    }

    /// Call when an agent deregisters.
    pub fn release() {
        let mut g = guard().lock().expect("power_guard lock poisoned");
        g.agent_count = g.agent_count.saturating_sub(1);
        if g.agent_count == 0 {
            if let Some(mut child) = g.process.take() {
                let _ = child.kill();
                let _ = child.wait();
                tracing::info!(
                    "power_guard: sleep inhibitor stopped (no agents)"
                );
            }
        }
    }

    /// Current status for API/diagnostics.
    pub fn status() -> PowerGuardStatus {
        let g = guard().lock().expect("power_guard lock poisoned");
        PowerGuardStatus {
            active: g.process.is_some(),
            agent_count: g.agent_count,
            platform: current_platform(),
        }
    }

    /// Reset internal state for test isolation.
    #[cfg(test)]
    fn reset() {
        let mut g = guard().lock().expect("power_guard lock poisoned");
        if let Some(mut child) = g.process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        g.agent_count = 0;
    }
}

fn current_platform() -> &'static str {
    #[cfg(target_os = "macos")]
    { "macos" }
    #[cfg(target_os = "linux")]
    { "linux" }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    { "unsupported" }
}

#[cfg(target_os = "macos")]
fn spawn_inhibitor() -> Option<Child> {
    Command::new("caffeinate")
        .args(["-i", "-d"])
        .spawn()
        .map_err(|e| tracing::warn!("power_guard: caffeinate failed: {e}"))
        .ok()
}

#[cfg(target_os = "linux")]
fn spawn_inhibitor() -> Option<Child> {
    Command::new("systemd-inhibit")
        .args([
            "--what=idle:sleep",
            "--who=convergio-daemon",
            "--why=Active agents",
            "--mode=block",
            "sleep",
            "infinity",
        ])
        .spawn()
        .map_err(|e| {
            tracing::warn!("power_guard: systemd-inhibit failed: {e}")
        })
        .ok()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn spawn_inhibitor() -> Option<Child> {
    tracing::debug!(
        "power_guard: no inhibitor available on this platform"
    );
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_release_counting() {
        PowerGuard::reset();
        let s = PowerGuard::status();
        assert_eq!(s.agent_count, 0);
        assert!(!s.active);

        PowerGuard::acquire();
        let s = PowerGuard::status();
        assert_eq!(s.agent_count, 1);
        // On macOS caffeinate should start
        #[cfg(target_os = "macos")]
        assert!(s.active);

        PowerGuard::acquire();
        assert_eq!(PowerGuard::status().agent_count, 2);

        PowerGuard::release();
        assert_eq!(PowerGuard::status().agent_count, 1);

        PowerGuard::release();
        let s = PowerGuard::status();
        assert_eq!(s.agent_count, 0);
        assert!(!s.active);
    }

    #[test]
    fn release_saturates_at_zero() {
        PowerGuard::reset();
        PowerGuard::release();
        PowerGuard::release();
        assert_eq!(PowerGuard::status().agent_count, 0);
        assert!(!PowerGuard::status().active);
    }
}
