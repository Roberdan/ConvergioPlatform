// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Config validation — helpful messages that tell users what to fix.

use super::ConvergioConfig;

const KNOWN_ROLES: &[&str] = &["standalone", "coordinator", "worker"];
const KNOWN_TRANSPORTS: &[&str] = &["lan", "tailscale", "manual"];
const KNOWN_DISCOVERY: &[&str] = &["mdns", "static", "tailscale"];

/// Validate a loaded config and return human-readable warnings/errors.
/// An empty Vec means everything looks good.
pub fn validate(config: &ConvergioConfig) -> Vec<String> {
    let mut issues: Vec<String> = Vec::new();

    // -- node --
    if !KNOWN_ROLES.contains(&config.node.role.as_str()) {
        issues.push(format!(
            "[node] role '{}' is not recognized. Expected one of: {}",
            config.node.role,
            KNOWN_ROLES.join(", ")
        ));
    }

    // -- daemon --
    if config.daemon.port < 1024 {
        issues.push(format!(
            "[daemon] port {} is below 1024. Use 1024-65535.",
            config.daemon.port
        ));
    }
    if let Some(ref tz) = config.daemon.timezone {
        if !looks_like_iana_tz(tz) {
            issues.push(format!(
                "[daemon] timezone '{}' does not look like a valid IANA timezone \
                 (expected format: Area/City, e.g. Europe/Rome).",
                tz
            ));
        }
    }
    if let Some(ref qh) = config.daemon.quiet_hours {
        if !looks_like_time_range(qh) {
            issues.push(format!(
                "[daemon] quiet_hours '{}' should be HH:MM-HH:MM (e.g. 23:00-07:00).",
                qh
            ));
        }
    }

    // -- mesh --
    if !KNOWN_TRANSPORTS.contains(&config.mesh.transport.as_str()) {
        issues.push(format!(
            "[mesh] transport '{}' is not recognized. Expected one of: {}",
            config.mesh.transport,
            KNOWN_TRANSPORTS.join(", ")
        ));
    }
    if !KNOWN_DISCOVERY.contains(&config.mesh.discovery.as_str()) {
        issues.push(format!(
            "[mesh] discovery '{}' is not recognized. Expected one of: {}",
            config.mesh.discovery,
            KNOWN_DISCOVERY.join(", ")
        ));
    }

    // -- kernel --
    if config.kernel.max_tokens == 0 {
        issues.push("[kernel] max_tokens must be > 0.".to_string());
    }

    issues
}

/// Quick heuristic: contains '/' and no spaces.
fn looks_like_iana_tz(s: &str) -> bool {
    s.contains('/') && !s.contains(' ') && s.len() >= 5
}

/// Check for HH:MM-HH:MM pattern.
fn looks_like_time_range(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return false;
    }
    parts.iter().all(|p| {
        let hm: Vec<&str> = p.split(':').collect();
        if hm.len() != 2 {
            return false;
        }
        let h = hm[0].parse::<u32>();
        let m = hm[1].parse::<u32>();
        matches!((h, m), (Ok(0..=23), Ok(0..=59)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConvergioConfig;

    #[test]
    fn valid_defaults_produce_no_warnings() {
        let cfg = ConvergioConfig::default();
        let issues = validate(&cfg);
        assert!(issues.is_empty(), "unexpected issues: {issues:?}");
    }

    #[test]
    fn bad_port_caught() {
        let mut cfg = ConvergioConfig::default();
        cfg.daemon.port = 80;
        let issues = validate(&cfg);
        assert!(issues.iter().any(|i| i.contains("port")));
    }

    #[test]
    fn bad_role_caught() {
        let mut cfg = ConvergioConfig::default();
        cfg.node.role = "boss".to_string();
        let issues = validate(&cfg);
        assert!(issues.iter().any(|i| i.contains("role")));
    }

    #[test]
    fn bad_timezone_caught() {
        let mut cfg = ConvergioConfig::default();
        cfg.daemon.timezone = Some("nope".to_string());
        let issues = validate(&cfg);
        assert!(issues.iter().any(|i| i.contains("timezone")));
    }

    #[test]
    fn bad_quiet_hours_caught() {
        let mut cfg = ConvergioConfig::default();
        cfg.daemon.quiet_hours = Some("midnight".to_string());
        let issues = validate(&cfg);
        assert!(issues.iter().any(|i| i.contains("quiet_hours")));
    }

    #[test]
    fn valid_quiet_hours_accepted() {
        let mut cfg = ConvergioConfig::default();
        cfg.daemon.quiet_hours = Some("23:00-07:00".to_string());
        let issues = validate(&cfg);
        assert!(
            !issues.iter().any(|i| i.contains("quiet_hours")),
            "unexpected: {issues:?}"
        );
    }

    #[test]
    fn bad_transport_caught() {
        let mut cfg = ConvergioConfig::default();
        cfg.mesh.transport = "carrier_pigeon".to_string();
        let issues = validate(&cfg);
        assert!(issues.iter().any(|i| i.contains("transport")));
    }

    #[test]
    fn zero_max_tokens_caught() {
        let mut cfg = ConvergioConfig::default();
        cfg.kernel.max_tokens = 0;
        let issues = validate(&cfg);
        assert!(issues.iter().any(|i| i.contains("max_tokens")));
    }
}
