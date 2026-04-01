// Route constants extension: PUT, DELETE, SSE, WS routes.

pub const PUT_ROUTES: &[&str] = &[
    "/api/ideas/:id",
    "/api/chat/requirement",
    "/api/peers/:name",
    "/api/nightly/config/:project_id",
    "/api/runs/:id",
];
pub const DELETE_ROUTES: &[&str] = &[
    "/api/ideas/:id",
    "/api/chat/session",
    "/api/peers/:name",
    "/api/delegate/:session_id",
    "/api/memory/forget/:id",
    "/api/memory-mgmt/file/:filename",
];
pub const SSE_ROUTES: &[&str] = &[
    "/api/chat/stream/:sid",
    "/api/mesh/action/stream",
    "/api/mesh/fullsync",
    "/api/plan/preflight",
    "/api/plan/delegate",
    "/api/plan/start",
    "/api/mesh/pull-db",
    "/api/a2ui/stream",
];
pub const WS_ROUTES: &[&str] = &["/ws/brain", "/ws/dashboard", "/ws/pty"];

#[cfg(test)]
mod tests {
    use super::super::api_routes::{GET_ROUTES, POST_ROUTES};
    use super::*;

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
