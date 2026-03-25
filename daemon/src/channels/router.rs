use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a single route entry.
/// `auto_approve_threshold` is a future hook: if an agent's approval rate
/// exceeds this value, notifications are suppressed (stub — not queried yet).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub channels: Vec<String>,
    #[serde(default)]
    pub auto_approve_threshold: Option<f64>,
}

/// Top-level routing configuration.
/// Maps event types to channel lists. "default" is the fallback key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    pub routing: HashMap<String, RouteConfig>,
}

/// Routes events to the appropriate notification channels.
pub struct ChannelRouter {
    routes: HashMap<String, RouteConfig>,
}

impl ChannelRouter {
    /// Build a router from a RoutingConfig.
    pub fn new(config: RoutingConfig) -> Self {
        Self {
            routes: config.routing,
        }
    }

    /// Build a router from an empty config (no routes).
    pub fn empty() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// Resolve which channels should receive a given event type.
    /// Falls back to "default" if the event type has no explicit route.
    /// Returns empty vec if neither the event nor "default" is configured.
    pub fn route(&self, event_type: &str) -> Vec<String> {
        if let Some(cfg) = self.routes.get(event_type) {
            return cfg.channels.clone();
        }
        if let Some(cfg) = self.routes.get("default") {
            return cfg.channels.clone();
        }
        Vec::new()
    }

    /// Fan-out: resolve channels for multiple event types at once.
    /// Deduplicates channel names across all events.
    pub fn fan_out(&self, event_types: &[&str]) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for et in event_types {
            for ch in self.route(et) {
                if seen.insert(ch.clone()) {
                    result.push(ch);
                }
            }
        }
        result
    }

    /// Check if auto-approve threshold is met for a route.
    /// Stub: returns the threshold if configured, None otherwise.
    /// Future: will query agent approval metrics.
    pub fn auto_approve_threshold(
        &self,
        event_type: &str,
    ) -> Option<f64> {
        self.routes
            .get(event_type)
            .and_then(|cfg| cfg.auto_approve_threshold)
    }
}

/// Deserialize a RoutingConfig from a JSON string.
pub fn parse_routing_config(json: &str) -> Result<RoutingConfig, String> {
    serde_json::from_str(json).map_err(|e| format!("routing config parse error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> RoutingConfig {
        let json = r#"{
            "routing": {
                "need_human": {
                    "channels": ["ntfy", "telegram"]
                },
                "plan_complete": {
                    "channels": ["ntfy"]
                },
                "wave_merged": {
                    "channels": ["ntfy"]
                },
                "agent_blocked": {
                    "channels": ["ntfy", "telegram"],
                    "auto_approve_threshold": 0.95
                },
                "default": {
                    "channels": ["ntfy"]
                }
            }
        }"#;
        parse_routing_config(json).expect("valid config")
    }

    #[test]
    fn route_lookup_returns_correct_channels() {
        let router = ChannelRouter::new(sample_config());
        let channels = router.route("need_human");
        assert_eq!(channels, vec!["ntfy", "telegram"]);

        let channels = router.route("plan_complete");
        assert_eq!(channels, vec!["ntfy"]);
    }

    #[test]
    fn route_falls_back_to_default() {
        let router = ChannelRouter::new(sample_config());
        let channels = router.route("unknown_event");
        assert_eq!(channels, vec!["ntfy"]);
    }

    #[test]
    fn empty_routes_returns_empty_vec() {
        let router = ChannelRouter::empty();
        let channels = router.route("need_human");
        assert!(channels.is_empty());
    }

    #[test]
    fn config_deserialization_roundtrip() {
        let json = r#"{
            "routing": {
                "need_human": {
                    "channels": ["ntfy", "telegram"],
                    "auto_approve_threshold": 0.9
                },
                "default": {
                    "channels": ["ntfy"]
                }
            }
        }"#;
        let config = parse_routing_config(json).expect("valid");
        assert_eq!(config.routing.len(), 2);

        let need_human = &config.routing["need_human"];
        assert_eq!(need_human.channels, vec!["ntfy", "telegram"]);
        assert_eq!(need_human.auto_approve_threshold, Some(0.9));

        let default = &config.routing["default"];
        assert_eq!(default.channels, vec!["ntfy"]);
        assert_eq!(default.auto_approve_threshold, None);
    }

    #[test]
    fn fan_out_deduplicates_channels() {
        let router = ChannelRouter::new(sample_config());
        // need_human=[ntfy, telegram], plan_complete=[ntfy]
        let channels = router.fan_out(&["need_human", "plan_complete"]);
        assert_eq!(channels, vec!["ntfy", "telegram"]);
    }

    #[test]
    fn auto_approve_threshold_returns_configured_value() {
        let router = ChannelRouter::new(sample_config());
        assert_eq!(
            router.auto_approve_threshold("agent_blocked"),
            Some(0.95)
        );
        assert_eq!(
            router.auto_approve_threshold("plan_complete"),
            None
        );
        assert_eq!(
            router.auto_approve_threshold("nonexistent"),
            None
        );
    }

    #[test]
    fn invalid_json_returns_error() {
        let result = parse_routing_config("not json");
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("routing config parse error")
        );
    }
}
