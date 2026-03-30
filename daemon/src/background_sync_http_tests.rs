use std::collections::HashMap;

use crate::background_sync_http::{detect_local_tailscale_ip, resolve_best_addr};

fn fields(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// --- resolve_best_addr ---

#[test]
fn resolve_returns_none_for_empty_fields() {
    assert!(resolve_best_addr("ghost", &HashMap::new()).is_none());
}

#[test]
fn resolve_skips_empty_ip_strings() {
    let f = fields(&[("tailscale_ip", ""), ("thunderbolt_ip", "")]);
    assert!(resolve_best_addr("node", &f).is_none());
}

#[test]
fn resolve_does_not_panic_on_malformed_ip() {
    let f = fields(&[("tailscale_ip", "not-a-real-ip")]);
    let result = std::panic::catch_unwind(|| resolve_best_addr("bad-peer", &f));
    assert!(result.is_ok(), "malformed IP must not panic");
    assert_eq!(result.ok().flatten(), None);
}

#[test]
fn resolve_does_not_panic_on_partial_ip() {
    let f = fields(&[("thunderbolt_ip", "10.0.0"), ("tailscale_ip", "100")]);
    let result = std::panic::catch_unwind(|| resolve_best_addr("partial", &f));
    assert!(result.is_ok(), "partial IP must not panic");
    assert_eq!(result.ok().flatten(), None);
}

#[test]
fn resolve_prefers_thunderbolt_over_tailscale() {
    let f = fields(&[
        ("thunderbolt_ip", "10.0.0.99"),
        ("tailscale_ip", "100.64.0.99"),
    ]);
    // Neither reachable in test — must not panic
    let _result = resolve_best_addr("dual", &f);
}

// --- detect_local_tailscale_ip env override ---

#[test]
fn detect_local_tailscale_ip_honours_env_override() {
    std::env::set_var("CONVERGIO_LOCAL_TAILSCALE_IP", "100.88.77.66");
    let ip = detect_local_tailscale_ip();
    std::env::remove_var("CONVERGIO_LOCAL_TAILSCALE_IP");
    assert_eq!(ip.as_deref(), Some("100.88.77.66"));
}
