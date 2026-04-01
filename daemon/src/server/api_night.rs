// Night status API — exposes current night mode state for the node.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use super::state::ServerState;
use crate::config;
use crate::config::night::is_night_hours;

#[derive(Debug, Serialize)]
struct NightStatus {
    is_night: bool,
    night_model: String,
    night_mode: bool,
    role: String,
}

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/node/night-status", get(handle_night_status))
}

async fn handle_night_status(
    State(_state): State<ServerState>,
) -> Json<NightStatus> {
    let cfg = config::load_config();
    let night_cfg = &cfg.night;
    let is_night = is_night_hours(night_cfg);
    Json(NightStatus {
        is_night,
        night_model: night_cfg.night_model.clone(),
        night_mode: night_cfg.night_mode,
        role: cfg.node.role.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::night::NightConfig;

    #[test]
    fn night_status_shape() {
        let status = NightStatus {
            is_night: true,
            night_model: "claude-haiku-4-5".to_string(),
            night_mode: true,
            role: "worker".to_string(),
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["is_night"], true);
        assert_eq!(json["night_model"], "claude-haiku-4-5");
        assert_eq!(json["night_mode"], true);
        assert_eq!(json["role"], "worker");
    }

    #[test]
    fn default_night_config_produces_response() {
        let cfg = NightConfig::default();
        let is_night = is_night_hours(&cfg);
        let status = NightStatus {
            is_night,
            night_model: cfg.night_model,
            night_mode: cfg.night_mode,
            role: "standalone".to_string(),
        };
        // night_mode is false by default, so is_night must be false
        assert!(!status.is_night);
        assert!(!status.night_mode);
    }
}
