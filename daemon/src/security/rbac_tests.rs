use super::*;
use crate::security::jwt::AgentRole;

#[test]
fn coordinator_accesses_everything() {
    let paths = [
        "/api/overview",
        "/api/mesh/init",
        "/api/plan-db/create",
        "/api/kernel/classify",
        "/api/agents",
        "/api/health",
    ];
    for path in paths {
        assert!(
            role_can_access(&AgentRole::Coordinator, path),
            "coordinator denied {path}"
        );
    }
}

#[test]
fn executor_allowed_task_and_build() {
    let allowed = [
        "/api/plan-db/task/update",
        "/api/plan-db/agent/start",
        "/api/plan-db/checkpoint/save",
        "/api/plan-db/list",
        "/api/build",
        "/api/test",
        "/api/health",
        "/api/tracking/tokens",
        "/api/workspace/create",
        "/api/ipc/send",
        "/api/delegate/spawn",
    ];
    for path in allowed {
        assert!(
            role_can_access(&AgentRole::Executor, path),
            "executor denied {path}"
        );
    }
}

#[test]
fn executor_denied_admin_routes() {
    let denied = [
        "/api/mesh/init",
        "/api/agents/create",
        "/api/coordinator/toggle",
        "/api/kernel/classify",
    ];
    for path in denied {
        assert!(
            !role_can_access(&AgentRole::Executor, path),
            "executor allowed {path}"
        );
    }
}

#[test]
fn kernel_allowed_own_routes() {
    let allowed = [
        "/api/kernel/status",
        "/api/kernel/classify",
        "/api/notify",
        "/api/health",
        "/api/node/readiness",
        "/api/heartbeat",
        "/api/voice/status",
    ];
    for path in allowed {
        assert!(
            role_can_access(&AgentRole::Kernel, path),
            "kernel denied {path}"
        );
    }
}

#[test]
fn kernel_denied_plan_routes() {
    let denied = [
        "/api/plan-db/create",
        "/api/agents/create",
        "/api/mesh/init",
    ];
    for path in denied {
        assert!(
            !role_can_access(&AgentRole::Kernel, path),
            "kernel allowed {path}"
        );
    }
}

#[test]
fn worker_allowed_read_and_status() {
    let allowed = [
        "/api/plan-db/task/update",
        "/api/plan-db/list",
        "/api/delegate/status",
        "/api/health",
        "/api/heartbeat",
        "/api/ipc/messages",
    ];
    for path in allowed {
        assert!(
            role_can_access(&AgentRole::Worker, path),
            "worker denied {path}"
        );
    }
}

#[test]
fn worker_denied_admin_routes() {
    let denied = [
        "/api/mesh/init",
        "/api/agents/create",
        "/api/coordinator/toggle",
        "/api/build",
    ];
    for path in denied {
        assert!(
            !role_can_access(&AgentRole::Worker, path),
            "worker allowed {path}"
        );
    }
}

#[test]
fn dashboard_wide_read_access() {
    let allowed = [
        "/api/overview",
        "/api/agents",
        "/api/plans/assignable",
        "/api/mesh/topology",
        "/api/health",
        "/api/runs",
        "/api/metrics/summary",
        "/ws/brain",
    ];
    for path in allowed {
        assert!(
            role_can_access(&AgentRole::Dashboard, path),
            "dashboard denied {path}"
        );
    }
}
