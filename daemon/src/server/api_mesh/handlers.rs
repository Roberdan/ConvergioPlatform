//! Core mesh API handlers: peers list, actions, and mesh admin operations.
use super::peer_conf::{detect_local_identity, is_local_peer_conf, parse_peers_conf};
use super::super::state::{query_rows, ApiError, ServerState};
use axum::extract::{Query, State};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::collections::HashMap;

pub(crate) async fn handle_delegate_cancel(
    axum::extract::Path(delegation_id): axum::extract::Path<String>,
) -> Json<Value> {
    let cancelled = super::super::sse_delegate::cancel_delegation(&delegation_id);
    if cancelled {
        Json(json!({"ok": true, "delegation_id": delegation_id, "status": "cancelled"}))
    } else {
        Json(json!({
            "ok": false, "delegation_id": delegation_id,
            "error": "delegation not found or already completed"
        }))
    }
}

pub(crate) async fn api_mesh(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT peer_name, last_seen, load_json, capabilities FROM peer_heartbeats \
         WHERE peer_name IS NOT NULL AND peer_name != '' \
         AND peer_name NOT LIKE '%.%.%.%:%'",
        [],
    )?;
    // Load peers.conf for static enrichment
    let conf_path = state
        .db_path
        .parent()
        .and_then(|d| d.parent())
        .map(|base| base.join("config/peers.conf"))
        .unwrap_or_default();
    let conf = std::fs::read_to_string(&conf_path).unwrap_or_default();
    let peer_conf = parse_peers_conf(&conf);
    let (local_host, local_ts_ip) = detect_local_identity();

    // Build lookup: peer_name -> heartbeat row
    let mut hb_map: HashMap<String, Value> = HashMap::new();
    for row in rows {
        if let Some(name) = row.get("peer_name").and_then(Value::as_str) {
            if !name.is_empty() {
                hb_map.insert(name.to_owned(), row);
            }
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    // Merge: peers.conf (authority) + heartbeat DB (dynamic)
    let mut peers: Vec<Value> = Vec::new();
    for (name, fields) in &peer_conf {
        let status = fields.get("status").map(|s| s.as_str()).unwrap_or("active");
        if status == "inactive" {
            continue;
        }
        let mut obj = serde_json::Map::new();
        obj.insert("peer_name".into(), json!(name));
        obj.insert("os".into(), json!(fields.get("os").cloned().unwrap_or_else(|| "unknown".into())));
        obj.insert("role".into(), json!(fields.get("role").cloned().unwrap_or_else(|| "worker".into())));
        obj.insert("capabilities".into(), json!(fields.get("capabilities").cloned().unwrap_or_default()));
        if let Some(ip) = fields.get("tailscale_ip") { obj.insert("tailscale_ip".into(), json!(ip)); }
        if let Some(dns) = fields.get("dns_name") { obj.insert("dns_name".into(), json!(dns)); }
        if let Some(alias) = fields.get("ssh_alias") { obj.insert("ssh_alias".into(), json!(alias)); }
        if let Some(mac) = fields.get("mac_address") { obj.insert("mac_address".into(), json!(mac)); }
        obj.insert("is_local".into(), json!(is_local_peer_conf(&local_host, &local_ts_ip, fields)));

        // Merge heartbeat dynamic data
        if let Some(hb) = hb_map.remove(name) {
            let seen = hb.get("last_seen").and_then(Value::as_f64).unwrap_or(0.0);
            obj.insert("last_seen".into(), json!(seen));
            obj.insert("is_online".into(), json!(now - seen < 3600.0));
            if let Some(load_str) = hb.get("load_json").and_then(Value::as_str) {
                if let Ok(load) = serde_json::from_str::<Value>(load_str) {
                    if let Some(load_obj) = load.as_object() {
                        for (k, v) in load_obj { obj.insert(k.clone(), v.clone()); }
                    }
                }
            }
            // DB capabilities override if richer
            if let Some(db_caps) = hb.get("capabilities").and_then(Value::as_str) {
                if !db_caps.is_empty() { obj.insert("capabilities".into(), json!(db_caps)); }
            }
        } else {
            obj.insert("is_online".into(), json!(false));
            obj.insert("last_seen".into(), json!(0));
        }
        if !obj.contains_key("cpu") { obj.insert("cpu".into(), json!(0)); }
        if !obj.contains_key("active_tasks") { obj.insert("active_tasks".into(), json!(0)); }
        let mut aliases: Vec<String> = Vec::new();
        if let Some(alias) = fields.get("ssh_alias") { aliases.push(alias.clone()); }
        if let Some(dns) = fields.get("dns_name") { aliases.push(dns.clone()); }
        obj.insert("hostname_aliases".into(), json!(aliases));
        peers.push(Value::Object(obj));
    }

    // Include any heartbeat-only peers not in peers.conf (shouldn't happen, but safe)
    for (name, mut hb) in hb_map {
        let seen = hb.get("last_seen").and_then(Value::as_f64).unwrap_or(0.0);
        let obj = hb.as_object_mut().unwrap();
        if !obj.contains_key("is_online") { obj.insert("is_online".into(), json!(now - seen < 3600.0)); }
        if !obj.contains_key("is_local") {
            obj.insert("is_local".into(), json!(name.to_lowercase().contains(&local_host)));
        }
        if !obj.contains_key("os") { obj.insert("os".into(), json!("unknown")); }
        if !obj.contains_key("cpu") { obj.insert("cpu".into(), json!(0)); }
        if !obj.contains_key("active_tasks") { obj.insert("active_tasks".into(), json!(0)); }
        if !obj.contains_key("role") {
            let role = if name.contains("mac-worker-2") || name.contains("local") { "coordinator" } else { "worker" };
            obj.insert("role".into(), json!(role));
        }
        if let Some(load_str) = obj.remove("load_json").and_then(|v| v.as_str().map(str::to_owned)) {
            if let Ok(load) = serde_json::from_str::<Value>(&load_str) {
                if let Some(load_obj) = load.as_object() {
                    for (k, v) in load_obj { obj.insert(k.clone(), v.clone()); }
                }
            }
        }
        peers.push(hb);
    }

    let daemon_ws = if !local_ts_ip.is_empty() {
        format!("ws://{}:9420/ws/brain", local_ts_ip)
    } else {
        String::new()
    };
    Ok(Json(json!({ "peers": peers, "daemon_ws": daemon_ws, "local_node": local_host })))
}

pub(crate) async fn api_mesh_init() -> Json<Value> {
    Json(json!({"status": "ok", "daemons_restarted": [], "hosts_needing_normalization": 0}))
}

pub(crate) async fn handle_mesh_action(
    Query(qs): Query<HashMap<String, String>>,
) -> Json<Value> {
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
                    if skip_section { continue; }
                }
                if skip_section && !trimmed.starts_with('[') { continue; }
                skip_section = false;
                result.push_str(line);
                result.push('\n');
            }
            match std::fs::write(&conf_path, &result) {
                Ok(_) => Json(json!({"ok": true, "output": format!("Removed {peer} from peers.conf")})),
                Err(e) => Json(json!({"error": format!("Failed to write: {e}")})),
            }
        }
        Err(e) => Json(json!({"error": format!("Failed to read peers.conf: {e}")})),
    }
}
