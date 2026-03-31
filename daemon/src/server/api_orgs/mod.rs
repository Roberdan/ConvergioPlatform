mod handlers;
mod decisions;
mod budget;
mod telemetry;
mod digest;
mod morning_brief;
#[cfg(test)]
mod integration_tests;

use super::state::ServerState;
use axum::routing::{delete, get, post, put};
use axum::Router;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/orgs", post(handlers::create_org))
        .route("/api/orgs", get(handlers::list_orgs))
        .route("/api/orgs/:id", get(handlers::get_org))
        .route("/api/orgs/:id", put(handlers::update_org))
        .route("/api/orgs/:id/members", post(handlers::add_member))
        .route(
            "/api/orgs/:id/members/:agent",
            delete(handlers::remove_member),
        )
        .route("/api/orgs/:id/services", post(handlers::register_service))
        .route("/api/orgs/:id/services", get(handlers::list_services))
        .route("/api/orgs/:id/decisions", post(decisions::log_decision))
        .route("/api/orgs/:id/decisions", get(decisions::list_decisions))
        .route("/api/orgs/:id/telemetry", post(telemetry::record_telemetry))
        .route("/api/orgs/:id/telemetry", get(telemetry::aggregate_telemetry))
        .route("/api/orgs/:id/telemetry/agents", get(telemetry::per_agent_telemetry))
        .route("/api/orgs/:id/digest", get(digest::latest_digest))
        .route("/api/orgs/:id/digest/generate", post(digest::generate_digest))
        .route("/api/digest/morning", get(morning_brief::get_morning_brief))
}

pub fn spawn_background_jobs(state: ServerState) {
    morning_brief::spawn_morning_brief_cron(state);
}
