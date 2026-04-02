// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// `cvg mesh join <coordinator_url>` — register this node with a mesh coordinator.

use crate::cli_error::CliError;
use crate::cli_setup_steps as steps;

/// Join the mesh by registering with the coordinator daemon.
/// Sends self-detected info, receives peers.conf + env, writes locally.
pub(crate) async fn handle_mesh_join(coordinator_url: &str) -> Result<(), CliError> {
    println!("Joining mesh via coordinator: {coordinator_url}");

    // Detect self info
    let node_name = steps::detect_hostname();
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let os = detect_os();
    let (tailscale_ip, dns_name) = detect_tailscale_self();
    if tailscale_ip.is_empty() {
        return Err(CliError::InvalidInput(
            "Tailscale not detected. Mesh requires Tailscale.".into(),
        ));
    }

    println!("  Node:     {node_name}");
    println!("  User:     {user}");
    println!("  OS:       {os}");
    println!("  IP:       {tailscale_ip}");
    println!("  DNS:      {dns_name}");

    // Build registration payload
    let body = serde_json::json!({
        "name": node_name,
        "ssh_alias": format!("{node_name}-ts"),
        "user": user,
        "os": os,
        "tailscale_ip": tailscale_ip,
        "dns_name": dns_name,
        "capabilities": detect_capabilities(),
        "role": "worker",
    });

    // POST to coordinator
    print!("  Registering with coordinator... ");
    let url = format!("{coordinator_url}/api/mesh/register");
    let client = reqwest::Client::new();
    let resp = client.post(&url).json(&body).send().await.map_err(|e| {
        CliError::ApiCallFailed(format!("cannot reach coordinator: {e}"))
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(CliError::ApiCallFailed(format!(
            "coordinator returned {status}: {text}"
        )));
    }

    let result: serde_json::Value = resp.json().await.map_err(|e| {
        CliError::ApiCallFailed(format!("invalid response: {e}"))
    })?;
    println!("OK");

    // Write peers.conf
    if let Some(peers_config) = result["peers_config"].as_str() {
        let path = peers_conf_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(CliError::Io)?;
        }
        std::fs::write(&path, peers_config).map_err(CliError::Io)?;
        println!("  peers.conf written to {}", path.display());
    }

    // Write env (merge, don't overwrite existing keys)
    if let Some(env_content) = result["env_content"].as_str() {
        if !env_content.is_empty() {
            merge_env_file(env_content)?;
            println!("  env file updated");
        }
    }

    println!();
    if let Some(msg) = result["message"].as_str() {
        println!("  {msg}");
    }
    println!();
    println!("Next steps:");
    println!("  cvg setup     — configure local node (if not done)");
    println!("  cvg serve     — start the daemon");
    println!("  cvg status    — verify mesh connectivity");
    Ok(())
}

fn detect_os() -> String {
    if cfg!(target_os = "macos") {
        "macos".to_string()
    } else if cfg!(target_os = "linux") {
        "linux".to_string()
    } else {
        std::env::consts::OS.to_string()
    }
}

fn detect_tailscale_self() -> (String, String) {
    let output = std::process::Command::new("tailscale")
        .args(["status", "--json"])
        .output();
    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return (String::new(), String::new()),
    };
    let json: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(v) => v,
        Err(_) => return (String::new(), String::new()),
    };
    let ip = json
        .pointer("/Self/TailscaleIPs/0")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let dns = json
        .pointer("/Self/DNSName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim_end_matches('.')
        .to_string();
    (ip, dns)
}

fn detect_capabilities() -> Vec<String> {
    let mut caps = Vec::new();
    if which("claude").is_some() {
        caps.push("claude".to_string());
    }
    if which("gh").is_some() {
        caps.push("copilot".to_string());
    }
    if which("ollama").is_some() {
        caps.push("ollama".to_string());
    }
    if caps.is_empty() {
        caps.push("worker".to_string());
    }
    caps
}

fn which(cmd: &str) -> Option<()> {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|_| ())
}

fn peers_conf_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home).join(".claude/config/peers.conf")
}

fn merge_env_file(new_content: &str) -> Result<(), CliError> {
    let path = std::path::PathBuf::from(
        std::env::var("HOME").unwrap_or_else(|_| ".".into()),
    )
    .join(".convergio/env");

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(CliError::Io)?;
    }

    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = existing.lines().map(String::from).collect();
    for line in new_content.lines() {
        let key = line.split_once('=').map(|(k, _)| k.trim());
        if let Some(k) = key {
            if !lines.iter().any(|l| l.starts_with(k)) {
                lines.push(line.to_string());
            }
        }
    }
    std::fs::write(&path, lines.join("\n") + "\n").map_err(CliError::Io)
}
