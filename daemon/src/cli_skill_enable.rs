// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Skill enable subcommand — reads skill.yaml, activates required agents and suggests plugins.

use crate::cli_skill_validate::{yaml_get_list, agent_name_valid};
use std::path::PathBuf;

/// Enable a skill: parse requires-agents + requires-plugins, activate agents, print summary.
pub async fn handle(skill_dir: &PathBuf, api_url: &str, human: bool) {
    let yaml_path = skill_dir.join("skill.yaml");
    if !yaml_path.is_file() {
        eprintln!("skill.yaml not found in {}", skill_dir.display());
        std::process::exit(2);
    }
    let yaml_content = match std::fs::read_to_string(&yaml_path) {
        Ok(c) => c,
        Err(e) => { eprintln!("read error: {e}"); std::process::exit(2); }
    };

    let agents = yaml_get_list(&yaml_content, "requires-agents").unwrap_or_default();
    let plugins = yaml_get_list(&yaml_content, "requires-plugins").unwrap_or_default();

    let mut agents_enabled: usize = 0;
    for agent_name in &agents {
        if !agent_name_valid(agent_name) {
            eprintln!("warning: skipping invalid agent name '{agent_name}'");
            continue;
        }
        let body = serde_json::json!({"name": agent_name, "target_dir": ".github/agents"});
        let url = format!("{api_url}/api/agents/enable");
        match reqwest::Client::new().post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                agents_enabled += 1;
                if human { println!("enabled agent: {agent_name}"); }
            }
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                eprintln!("warning: failed to enable agent '{agent_name}': {status} {text}");
            }
            Err(e) => {
                eprintln!("warning: failed to enable agent '{agent_name}': {e}");
            }
        }
    }

    if human {
        if !plugins.is_empty() {
            println!("\nSuggested plugins (install manually):");
            for plugin in &plugins {
                println!("  - {plugin}");
            }
        }
        println!("\nSummary: {agents_enabled} agents enabled, {} plugins suggested", plugins.len());
    } else {
        println!("{}", serde_json::json!({
            "ok": true,
            "agents_enabled": agents_enabled,
            "agents_total": agents.len(),
            "plugins_suggested": plugins,
        }));
    }
}
