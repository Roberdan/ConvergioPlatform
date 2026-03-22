/// All registered API route paths — used by middleware, tests, and tooling.
pub const GET_ROUTES: &[&str] = &[
    "/api/ideas",
    "/api/ideas/:id",
    "/api/ideas/:id/notes",
    "/api/overview",
    "/api/mission",
    "/api/tokens/daily",
    "/api/tokens/models",
    "/api/mesh",
    "/api/mesh/logs",
    "/api/mesh/metrics",
    "/api/mesh/sync-stats",
    "/api/mesh/sync-status",
    "/api/mesh/traffic",
    "/api/mesh/provision",
    "/api/history",
    "/api/tasks/distribution",
    "/api/tasks/blocked",
    "/api/plans/assignable",
    "/api/notifications",
    "/api/nightly/jobs",
    "/api/nightly/config/:project_id",
    "/api/nightly/jobs/:id",
    "/api/projects",
    "/api/events",
    "/api/coordinator/status",
    "/api/coordinator/toggle",
    "/api/health",
    "/api/peers",
    "/api/peers/discover",
    "/api/agents",
    "/api/sessions",
    "/api/chat/models",
    "/api/chat/sessions",
    "/api/optimize/signals",
    "/api/ipc/agents",
    "/api/ipc/messages",
    "/api/ipc/channels",
    "/api/ipc/context",
    "/api/ipc/locks",
    "/api/ipc/worktrees",
    "/api/ipc/conflicts",
    "/api/ipc/status",
    "/api/ipc/budget",
    "/api/ipc/models",
    "/api/ipc/skills",
    "/api/ipc/auth-status",
    "/api/ipc/route-history",
    "/api/plan-db/context/:plan_id",
    "/api/plan-db/json/:plan_id",
    "/api/plan-db/list",
    "/api/plan-db/execution-tree/:plan_id",
    "/api/plan-db/drift-check/:plan_id",
    "/api/plan-db/validate-task/:task_id/:plan_id",
    "/api/plan-db/kb-search",
    "/api/plan-db/readiness/:plan_id",
    "/api/plan-db/review/check",
    "/api/plan-db/checkpoint/restore",
    "/api/peers/coordinator",
    "/api/mesh/topology",
    "/api/mesh/ping/:peer",
    "/api/mesh/diagnostics",
    "/api/notify/queue",
    "/api/heartbeat/status",
    "/api/watchdog/status",
    "/api/watchdog/diagnostics",
    "/api/coordinator/events",
    "/api/workers",
    "/api/workers/status",
    "/api/evolution/proposals",
    "/api/evolution/experiments",
    "/api/evolution/roi",
    "/api/evolution/audit/:id",
    "/api/metrics/run/:id",
    "/api/metrics/summary",
    "/api/metrics/cost",
    "/api/runs",
    "/api/runs/:id",
    "/api/ingest/formats",
];
pub const POST_ROUTES: &[&str] = &[
    "/api/ideas",
    "/api/ideas/:id/notes",
    "/api/ideas/:id/promote",
    "/api/chat/session",
    "/api/chat/message",
    "/api/chat/approve",
    "/api/chat/execute",
    "/api/github/repo/create",
    "/api/mesh/init",
    "/api/nightly/jobs/create",
    "/api/nightly/jobs/trigger",
    "/api/nightly/jobs/definitions/:id/toggle",
    "/api/nightly/jobs/:id/retry",
    "/api/projects",
    "/api/plan-status",
    "/api/peers",
    "/api/peers/ssh-check",
    "/api/plans/:plan_id/validate",
    "/api/optimize/clear",
    "/api/ipc/send",
    "/api/plan-db/task/update",
    "/api/plan-db/agent/start",
    "/api/plan-db/agent/complete",
    "/api/plan-db/create",
    "/api/plan-db/start/:plan_id",
    "/api/plan-db/complete/:plan_id",
    "/api/plan-db/cancel/:plan_id",
    "/api/plan-db/approve/:plan_id",
    "/api/plan-db/import",
    "/api/plan-db/review/register",
    "/api/plan-db/review/reset",
    "/api/plan-db/checkpoint/save",
    "/api/plan-db/kb-write",
    "/api/plan-db/wave/update",
    "/api/notify",
    "/api/notify/deliver",
    "/api/heartbeat",
    "/api/coordinator/emit",
    "/api/coordinator/process",
    "/api/mesh/exec",
    "/api/mesh/delegate",
    "/api/mesh/delegate/:id/cancel",
    "/api/workers/launch",
    "/api/evolution/proposals/:id/approve",
    "/api/evolution/proposals/:id/reject",
    "/api/runs",
    "/api/runs/:id/pause",
    "/api/runs/:id/resume",
    "/api/ingest",
    "/api/tracking/tokens",
    "/api/tracking/agent-activity",
    "/api/tracking/session-state",
    "/api/tracking/compaction",
];
pub const PUT_ROUTES: &[&str] = &[
    "/api/ideas/:id",
    "/api/chat/requirement",
    "/api/peers/:name",
    "/api/nightly/config/:project_id",
    "/api/runs/:id",
];
pub const DELETE_ROUTES: &[&str] = &["/api/ideas/:id", "/api/chat/session", "/api/peers/:name"];
pub const SSE_ROUTES: &[&str] = &[
    "/api/chat/stream/:sid",
    "/api/mesh/action/stream",
    "/api/mesh/fullsync",
    "/api/plan/preflight",
    "/api/plan/delegate",
    "/api/plan/start",
    "/api/mesh/pull-db",
];
pub const WS_ROUTES: &[&str] = &["/ws/brain", "/ws/dashboard", "/ws/pty"];

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct RateLimiter {
    pub(super) buckets: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl RateLimiter {
    pub(super) async fn allow(&self, category: String, limit: usize, window: Duration) -> bool {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().await;
        let entries = buckets.entry(category).or_default();
        entries.retain(|seen| now.duration_since(*seen) <= window);
        if entries.len() >= limit {
            return false;
        }
        entries.push(now);
        true
    }
}

pub fn endpoint_category(path: &str) -> String {
    let mut segments = path.split('/').filter(|segment| !segment.is_empty());
    match (segments.next(), segments.next()) {
        (Some("api"), Some(category)) => format!("api:{category}"),
        (Some(segment), _) => segment.to_string(),
        _ => "root".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{GET_ROUTES, POST_ROUTES, SSE_ROUTES, WS_ROUTES};

    #[test]
    fn includes_http_ws_and_sse_routes() {
        assert!(POST_ROUTES.contains(&"/api/mesh/init"));
        assert!(SSE_ROUTES.contains(&"/api/chat/stream/:sid"));
        assert!(WS_ROUTES.contains(&"/ws/brain"));
        assert!(WS_ROUTES.contains(&"/ws/dashboard"));
    }

    #[test]
    fn includes_ported_get_routes() {
        assert!(GET_ROUTES.contains(&"/api/overview"));
        assert!(GET_ROUTES.contains(&"/api/chat/sessions"));
        assert!(GET_ROUTES.contains(&"/api/projects"));
        assert!(GET_ROUTES.contains(&"/api/nightly/jobs/:id"));
        assert!(GET_ROUTES.contains(&"/api/nightly/config/:project_id"));
        assert!(POST_ROUTES.contains(&"/api/nightly/jobs/trigger"));
        assert!(POST_ROUTES.contains(&"/api/nightly/jobs/:id/retry"));
        assert!(POST_ROUTES.contains(&"/api/nightly/jobs/definitions/:id/toggle"));
    }
}
