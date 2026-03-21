// api_dashboard: dashboard API routes — overview, plans, nightly jobs, optimize
mod nightly;
mod nightly_data;
mod nightly_handlers;
mod overview;
mod plans;
mod plans_detail;

use super::state::ServerState;
use axum::routing::{get, post};
use axum::Router;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/overview", get(overview::api_overview))
        .route("/api/mission", get(plans::api_mission))
        .route("/api/tokens/daily", get(plans::api_tokens_daily))
        .route("/api/tokens/models", get(plans::api_tokens_models))
        .route("/api/history", get(plans::api_history))
        .route("/api/missions/recent", get(plans::api_recent_missions))
        .route("/api/tasks/distribution", get(plans::api_tasks_distribution))
        .route("/api/tasks/blocked", get(plans::api_tasks_blocked))
        .route("/api/plans/assignable", get(plans::api_plans_assignable))
        .route("/api/notifications", get(plans::api_notifications))
        .route("/api/nightly/jobs/trigger", post(nightly::api_nightly_job_trigger))
        .route(
            "/api/nightly/jobs/definitions/:id/toggle",
            post(nightly::api_nightly_def_toggle),
        )
        .route("/api/nightly/jobs", get(nightly::api_nightly_jobs))
        .route("/api/nightly/jobs/create", post(nightly::api_nightly_job_create))
        .route("/api/nightly/jobs/:id/retry", post(nightly::api_nightly_job_retry))
        .route("/api/nightly/jobs/:id", get(nightly::api_nightly_job_detail))
        .route(
            "/api/nightly/config/:project_id",
            get(nightly::api_nightly_config_get).put(nightly::api_nightly_config_update),
        )
        .route("/api/projects", get(plans::api_projects).post(plans::api_project_create))
        .route("/api/events", get(nightly::api_events))
        .route("/api/coordinator/status", get(nightly::api_coordinator_status))
        .route("/api/coordinator/toggle", get(nightly::api_coordinator_toggle))
        .route("/api/plan/:plan_id", get(plans::api_plan_detail))
        .route("/api/plan-status", post(plans::api_plan_status))
        .route("/api/optimize/signals", get(nightly::api_optimize_signals))
        .route("/api/optimize/clear", post(nightly::api_optimize_clear))
        .route("/api/plans/timeline", get(plans::api_plans_timeline))
}
