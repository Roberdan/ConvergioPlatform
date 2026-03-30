//! Route-level RBAC for agent roles.
//!
//! Defines which [`AgentRole`] can access which API endpoint prefixes.
//! The coordinator has unrestricted access; other roles are allow-listed.

use super::jwt::AgentRole;

/// Check if a role is allowed to access the given route path.
pub fn role_can_access(role: &AgentRole, path: &str) -> bool {
    match role {
        AgentRole::Coordinator => true,
        AgentRole::Dashboard => is_dashboard_route(path),
        AgentRole::Executor => is_executor_route(path),
        AgentRole::Kernel => is_kernel_route(path),
        AgentRole::Worker => is_worker_route(path),
    }
}

fn is_executor_route(path: &str) -> bool {
    // Executors: task ops, plan reads, build, test, checkpoints
    path.starts_with("/api/plan-db/task/")
        || path.starts_with("/api/plan-db/agent/")
        || path.starts_with("/api/plan-db/checkpoint/")
        || path.starts_with("/api/plan-db/context/")
        || path.starts_with("/api/plan-db/json/")
        || path.starts_with("/api/plan-db/list")
        || path.starts_with("/api/plan-db/execution-tree/")
        || path.starts_with("/api/plan-db/readiness/")
        || path.starts_with("/api/plan-db/kb")
        || path == "/api/build"
        || path == "/api/test"
        || path.starts_with("/api/build/")
        || path.starts_with("/api/test/")
        || path == "/api/health"
        || path.starts_with("/api/health/")
        || path.starts_with("/api/tracking/")
        || path.starts_with("/api/workspace/")
        || path.starts_with("/api/memory/")
        || path.starts_with("/api/ipc/")
        || path.starts_with("/api/delegate/")
        || path.starts_with("/api/delegation/")
        || path == "/api/notify"
        || path.starts_with("/api/notify/")
}

fn is_kernel_route(path: &str) -> bool {
    path.starts_with("/api/kernel/")
        || path == "/api/notify"
        || path.starts_with("/api/notify/")
        || path == "/api/health"
        || path.starts_with("/api/health/")
        || path.starts_with("/api/node/")
        || path.starts_with("/api/heartbeat")
        || path.starts_with("/api/voice/")
        || path.starts_with("/api/memory/")
}

fn is_worker_route(path: &str) -> bool {
    // Workers: read tasks, delegate status, heartbeat
    path.starts_with("/api/plan-db/task/")
        || path.starts_with("/api/plan-db/list")
        || path.starts_with("/api/plan-db/context/")
        || path.starts_with("/api/delegate/status")
        || path.starts_with("/api/delegation/")
        || path == "/api/health"
        || path.starts_with("/api/health/")
        || path.starts_with("/api/heartbeat")
        || path.starts_with("/api/ipc/")
        || path.starts_with("/api/tracking/")
}

fn is_dashboard_route(path: &str) -> bool {
    // Dashboard: read-only access to most GET endpoints
    path == "/api/health"
        || path.starts_with("/api/health/")
        || path.starts_with("/api/overview")
        || path.starts_with("/api/ideas")
        || path.starts_with("/api/agents")
        || path.starts_with("/api/sessions")
        || path.starts_with("/api/plans")
        || path.starts_with("/api/plan-db/")
        || path.starts_with("/api/mesh")
        || path.starts_with("/api/tokens/")
        || path.starts_with("/api/tasks/")
        || path.starts_with("/api/notifications")
        || path.starts_with("/api/projects")
        || path.starts_with("/api/events")
        || path.starts_with("/api/coordinator/")
        || path.starts_with("/api/peers")
        || path.starts_with("/api/chat/")
        || path.starts_with("/api/runs")
        || path.starts_with("/api/metrics/")
        || path.starts_with("/api/evolution/")
        || path.starts_with("/api/workspace/")
        || path.starts_with("/api/ipc/")
        || path.starts_with("/api/kernel/")
        || path.starts_with("/api/node/")
        || path.starts_with("/api/memory")
        || path.starts_with("/api/audit/")
        || path.starts_with("/api/nightly/")
        || path.starts_with("/ws/")
}

#[cfg(test)]
#[path = "rbac_tests.rs"]
mod tests;
