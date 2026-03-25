//! Mesh admin handlers: node add/remove actions.
use axum::extract::Query;
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Reject INI-unsafe characters: newline, `[`, `]`, `=`.
/// These would allow injecting new sections or key=value pairs into peers.conf.
fn validate_ini_field(value: &str, field: &str) -> Result<(), Json<Value>> {
    if value.chars().any(|c| matches!(c, '\n' | '\r' | '[' | ']' | '=')) {
        Err(Json(
            json!({"error": format!("invalid characters in field '{field}': newline, '[', ']', '=' are not allowed")}),
        ))
    } else {
        Ok(())
    }
}

pub(crate) async fn handle_mesh_action(Query(qs): Query<HashMap<String, String>>) -> Json<Value> {
    let action = qs.get("action").cloned().unwrap_or_default();
    let peer = qs.get("peer").cloned().unwrap_or_default();
    if action.is_empty() || peer.is_empty() {
        return Json(json!({"error": "missing action or peer", "output": ""}));
    }
    match action.as_str() {
        "add-node" => handle_add_node(&peer, &qs),
        "remove-node" => handle_remove_node(&peer),
        _ => Json(json!({"output": format!("{action} -> {peer}"), "exit_code": 0})),
    }
}

fn handle_add_node(peer: &str, qs: &HashMap<String, String>) -> Json<Value> {
    let ip = qs.get("ip").cloned().unwrap_or_default();
    let os = qs.get("os").cloned().unwrap_or("linux".into());
    let role = qs.get("role").cloned().unwrap_or("worker".into());
    let caps = qs.get("caps").cloned().unwrap_or("claude,copilot".into());
    let ssh = qs.get("ssh").cloned().unwrap_or_default();
    if ip.is_empty() {
        return Json(json!({"error": "Tailscale IP is required"}));
    }
    // Validate all fields that will be written into the INI file to prevent injection.
    for (val, name) in [
        (peer, "peer"),
        (ssh.as_str(), "ssh"),
        (os.as_str(), "os"),
        (caps.as_str(), "caps"),
        (role.as_str(), "role"),
        (ip.as_str(), "ip"),
    ] {
        if let Err(e) = validate_ini_field(val, name) {
            return e;
        }
    }
    let conf_path = std::env::var("HOME").unwrap_or_default() + "/.claude/config/peers.conf";
    let entry = format!(
        "\n[{peer}]\nssh_alias={ssh}\nos={os}\ntailscale_ip={ip}\ncapabilities={caps}\nrole={role}\nstatus=active\n"
    );
    match std::fs::OpenOptions::new().append(true).open(&conf_path) {
        Ok(mut f) => {
            use std::io::Write;
            let _ = f.write_all(entry.as_bytes());
            Json(json!({"ok": true, "output": format!("Added {peer} ({ip}) to peers.conf")}))
        }
        Err(e) => Json(json!({"error": format!("Failed to write peers.conf: {e}")})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_ini_field_accepts_clean_values() {
        assert!(validate_ini_field("worker-node-1", "peer").is_ok());
        assert!(validate_ini_field("linux", "os").is_ok());
        assert!(validate_ini_field("claude,copilot", "caps").is_ok());
        assert!(validate_ini_field("worker", "role").is_ok());
        assert!(validate_ini_field("100.64.0.1", "ip").is_ok());
    }

    #[test]
    fn validate_ini_field_rejects_newline() {
        assert!(validate_ini_field("bad\nvalue", "peer").is_err());
        assert!(validate_ini_field("bad\rvalue", "peer").is_err());
    }

    #[test]
    fn validate_ini_field_rejects_bracket_injection() {
        // Attacker could close the current section and open a new one
        assert!(validate_ini_field("node]\n[evil", "peer").is_err());
        assert!(validate_ini_field("[evil]", "os").is_err());
    }

    #[test]
    fn validate_ini_field_rejects_equals_injection() {
        // Attacker could inject extra key=value pairs
        assert!(validate_ini_field("val\nstatus=compromised", "role").is_err());
        assert!(validate_ini_field("key=inject", "caps").is_err());
    }
}

fn handle_remove_node(peer: &str) -> Json<Value> {
    let conf_path = std::env::var("HOME").unwrap_or_default() + "/.claude/config/peers.conf";
    match std::fs::read_to_string(&conf_path) {
        Ok(content) => {
            let mut result = String::new();
            let mut skip_section = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    let section = &trimmed[1..trimmed.len() - 1];
                    skip_section = section == peer;
                    if skip_section {
                        continue;
                    }
                }
                if skip_section && !trimmed.starts_with('[') {
                    continue;
                }
                skip_section = false;
                result.push_str(line);
                result.push('\n');
            }
            match std::fs::write(&conf_path, &result) {
                Ok(_) => {
                    Json(json!({"ok": true, "output": format!("Removed {peer} from peers.conf")}))
                }
                Err(e) => Json(json!({"error": format!("Failed to write: {e}")})),
            }
        }
        Err(e) => Json(json!({"error": format!("Failed to read peers.conf: {e}")})),
    }
}
