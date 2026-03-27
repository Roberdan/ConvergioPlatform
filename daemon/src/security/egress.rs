use super::types::SecurityError;
use std::collections::HashMap;
use std::sync::Mutex;

/// Per-agent egress firewall. Intercepts outbound connections.
pub struct EgressFirewall {
    /// Per-agent allowlists: agent_id → set of allowed patterns.
    allowlists: Mutex<HashMap<String, Vec<String>>>,
    /// Blocked connection log.
    blocked_log: Mutex<Vec<EgressEvent>>,
}

#[derive(Debug, Clone)]
pub struct EgressEvent {
    pub agent_id: String,
    pub destination: String,
    pub allowed: bool,
    pub timestamp: String,
}

impl EgressFirewall {
    pub fn new() -> Self {
        Self {
            allowlists: Mutex::new(HashMap::new()),
            blocked_log: Mutex::new(Vec::new()),
        }
    }

    /// Set allowed destinations for an agent. Patterns: exact host:port or *.domain.com.
    pub fn set_allowlist(&self, agent_id: &str, patterns: Vec<String>) {
        if let Ok(mut lists) = self.allowlists.lock() {
            lists.insert(agent_id.to_string(), patterns);
        }
    }

    /// Check if an agent can connect to a destination.
    pub fn check(&self, agent_id: &str, destination: &str) -> Result<(), SecurityError> {
        let lists = self.allowlists.lock()
            .map_err(|e| SecurityError::ConfigError(format!("lock: {e}")))?;

        let patterns = lists.get(agent_id).ok_or_else(|| {
            self.log_event(agent_id, destination, false);
            SecurityError::AccessDenied(format!("{agent_id}: no egress allowlist configured"))
        })?;

        let allowed = patterns.iter().any(|p| matches_egress(p, destination));
        self.log_event(agent_id, destination, allowed);

        if allowed {
            Ok(())
        } else {
            Err(SecurityError::AccessDenied(format!(
                "{agent_id}: egress to {destination} blocked"
            )))
        }
    }

    /// Get recent blocked events.
    pub fn blocked_events(&self) -> Vec<EgressEvent> {
        self.blocked_log.lock()
            .map(|l| l.iter().filter(|e| !e.allowed).cloned().collect())
            .unwrap_or_default()
    }

    fn log_event(&self, agent_id: &str, dest: &str, allowed: bool) {
        if let Ok(mut log) = self.blocked_log.lock() {
            log.push(EgressEvent {
                agent_id: agent_id.to_string(),
                destination: dest.to_string(),
                allowed,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
            // Keep log bounded.
            if log.len() > 10_000 {
                log.drain(..5_000);
            }
        }
    }
}

impl Default for EgressFirewall {
    fn default() -> Self {
        Self::new()
    }
}

fn matches_egress(pattern: &str, destination: &str) -> bool {
    if pattern == "*" { return true; }
    if let Some(domain) = pattern.strip_prefix("*.") {
        return destination.contains(domain);
    }
    pattern == destination
}
