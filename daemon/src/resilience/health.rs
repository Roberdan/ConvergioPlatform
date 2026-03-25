//! Health checking infrastructure for daemon components.
//!
//! Every component implements HealthCheck to expose its status.
//! A HealthRegistry aggregates all registered components for
//! the /health endpoint and degraded-mode detection.

use std::sync::{Arc, Mutex};

/// Overall health classification for a component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// Operating normally.
    Healthy,
    /// Partially impaired; non-critical degradation.
    Degraded,
    /// Not functioning; requests will fail.
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Point-in-time health snapshot for a single component.
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    /// Component identifier.
    pub name: String,
    /// Current health status.
    pub status: HealthStatus,
    /// Optional human-readable detail (e.g. error message).
    pub message: Option<String>,
}

/// Trait implemented by every checkable daemon component.
pub trait HealthCheck: Send + Sync {
    /// Returns the component's stable identifier.
    fn name(&self) -> &str;
    /// Performs a synchronous health probe and returns current state.
    fn check(&self) -> ComponentHealth;
}

/// Thread-safe registry that aggregates health checks.
///
/// Use `Arc<HealthRegistry>` when sharing across threads/tasks.
pub struct HealthRegistry {
    checks: Mutex<Vec<Arc<dyn HealthCheck>>>,
}

impl HealthRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            checks: Mutex::new(Vec::new()),
        }
    }

    /// Registers a component. Panics only if the internal lock is poisoned.
    pub fn register(&self, check: Arc<dyn HealthCheck>) {
        self.checks.lock().expect("registry lock poisoned").push(check);
    }

    /// Returns health snapshots for all registered components.
    pub fn check_all(&self) -> Vec<ComponentHealth> {
        self.checks
            .lock()
            .expect("registry lock poisoned")
            .iter()
            .map(|c| c.check())
            .collect()
    }

    /// Returns the aggregate status:
    /// - Any Unhealthy component → Unhealthy
    /// - Any Degraded component  → Degraded
    /// - Otherwise               → Healthy
    pub fn aggregate_status(&self) -> HealthStatus {
        let checks = self.check_all();
        if checks.iter().any(|c| c.status == HealthStatus::Unhealthy) {
            return HealthStatus::Unhealthy;
        }
        if checks.iter().any(|c| c.status == HealthStatus::Degraded) {
            return HealthStatus::Degraded;
        }
        HealthStatus::Healthy
    }
}

impl Default for HealthRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    struct FakeComponent(HealthStatus);

    impl HealthCheck for FakeComponent {
        fn name(&self) -> &str {
            "fake"
        }

        fn check(&self) -> ComponentHealth {
            ComponentHealth {
                name: self.name().to_string(),
                status: self.0.clone(),
                message: None,
            }
        }
    }

    #[test]
    fn registry_aggregate_all_healthy() {
        let reg = HealthRegistry::new();
        reg.register(Arc::new(FakeComponent(HealthStatus::Healthy)));
        reg.register(Arc::new(FakeComponent(HealthStatus::Healthy)));
        assert_eq!(reg.aggregate_status(), HealthStatus::Healthy);
    }

    #[test]
    fn registry_aggregate_one_degraded() {
        let reg = HealthRegistry::new();
        reg.register(Arc::new(FakeComponent(HealthStatus::Healthy)));
        reg.register(Arc::new(FakeComponent(HealthStatus::Degraded)));
        assert_eq!(reg.aggregate_status(), HealthStatus::Degraded);
    }

    #[test]
    fn registry_aggregate_one_unhealthy() {
        let reg = HealthRegistry::new();
        reg.register(Arc::new(FakeComponent(HealthStatus::Degraded)));
        reg.register(Arc::new(FakeComponent(HealthStatus::Unhealthy)));
        assert_eq!(reg.aggregate_status(), HealthStatus::Unhealthy);
    }

    #[test]
    fn registry_check_all_returns_all_components() {
        let reg = HealthRegistry::new();
        reg.register(Arc::new(FakeComponent(HealthStatus::Healthy)));
        reg.register(Arc::new(FakeComponent(HealthStatus::Degraded)));
        assert_eq!(reg.check_all().len(), 2);
    }
}
