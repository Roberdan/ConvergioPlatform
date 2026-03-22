// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Skill lint+transpile subcommands — replaces skill-lint.sh and skill-transpile-*.sh.
use clap::Subcommand;
use std::path::PathBuf;

// Re-export validation helpers so transpile module and tests keep working
pub(crate) use crate::cli_skill_validate::{
    capitalise, name_format_valid, semver_ge, strip_h1, version_format_valid,
    yaml_get, yaml_get_list,
};

pub(crate) const MIN_CONSTITUTION_VERSION: &str = "2.0.0";
pub(crate) const TOKEN_BUDGET_BYTES: u64 = 6144;
const REQUIRED_FIELDS: &[&str] = &[
    "name", "version", "description", "domain",
    "constitution-version", "license", "copyright",
];

#[derive(Debug, Subcommand)]
pub enum SkillCommands {
    /// Validate a skill directory (skill.yaml + SKILL.md)
    Lint {
        /// Path to the skill directory (must contain skill.yaml and SKILL.md)
        skill_dir: PathBuf,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
        /// Lint all subdirectories inside this directory
        #[arg(long)]
        all: bool,
    },
    /// Transpile skill to provider format
    Transpile {
        /// Path to the skill directory
        skill_dir: PathBuf,
        /// Output directory (default: current directory)
        #[arg(long, default_value = ".")]
        output_dir: PathBuf,
        /// Target provider: claude-code, copilot-cli, generic-llm
        #[arg(long, default_value = "claude-code")]
        provider: String,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
    },
    /// Enable a skill and auto-activate its required agents/plugins
    Enable {
        /// Path to the skill directory
        skill_dir: PathBuf,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
    },
}

pub async fn handle(cmd: SkillCommands) {
    match cmd {
        SkillCommands::Lint { skill_dir, human, all } => {
            handle_lint(&skill_dir, human, all);
        }
        SkillCommands::Transpile { skill_dir, output_dir, provider, human } => {
            crate::cli_skill_transpile::handle_transpile(&skill_dir, &output_dir, &provider, human);
        }
        SkillCommands::Enable { skill_dir, api_url, human } => {
            crate::cli_skill_enable::handle(&skill_dir, &api_url, human).await;
        }
    }
}

// -- Lint --------------------------------------------------------------------

fn handle_lint(skill_dir: &PathBuf, human: bool, all: bool) {
    if all {
        let entries = match std::fs::read_dir(skill_dir) {
            Ok(e) => e,
            Err(err) => {
                eprintln!("error reading directory {}: {err}", skill_dir.display());
                std::process::exit(2);
            }
        };
        let mut results = Vec::new();
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                results.push(lint_one(&entry.path()));
            }
        }
        if results.is_empty() {
            eprintln!("no skill directories found in {}", skill_dir.display());
            std::process::exit(1);
        }
        let pass = results.iter().filter(|r| r.ok).count();
        let fail = results.iter().filter(|r| !r.ok).count();
        if human {
            for r in &results {
                for msg in &r.messages { println!("{msg}"); }
            }
            println!("\nSummary: {pass} passed, {fail} failed");
        } else {
            println!("{}", serde_json::json!({
                "results": results.iter().map(|r| r.to_json()).collect::<Vec<_>>(),
                "summary": {"pass": pass, "fail": fail}
            }));
        }
        if fail > 0 { std::process::exit(1); }
    } else {
        let result = lint_one(skill_dir);
        let pass: usize = if result.ok { 1 } else { 0 };
        let fail = 1 - pass;
        if human {
            for msg in &result.messages { println!("{msg}"); }
            println!("\nSummary: {pass} passed, {fail} failed");
        } else {
            println!("{}", serde_json::json!({
                "results": [result.to_json()],
                "summary": {"pass": pass, "fail": fail}
            }));
        }
        if !result.ok { std::process::exit(1); }
    }
}

pub(crate) struct LintResult {
    pub(crate) skill: String,
    pub(crate) ok: bool,
    pub(crate) messages: Vec<String>,
}
impl LintResult {
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::json!({"skill": self.skill, "ok": self.ok, "messages": self.messages})
    }
}

pub(crate) fn lint_one(skill_dir: &PathBuf) -> LintResult {
    let name = skill_dir.file_name()
        .and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
    let yaml_path = skill_dir.join("skill.yaml");
    let md_path = skill_dir.join("SKILL.md");
    let mut msgs: Vec<String> = Vec::new();
    let mut failed = false;

    if yaml_path.is_file() {
        msgs.push(format!("[PASS] {name}: skill.yaml exists"));
    } else {
        msgs.push(format!("[FAIL] {name}: skill.yaml missing"));
        failed = true;
    }

    if yaml_path.is_file() {
        let yaml_content = std::fs::read_to_string(&yaml_path).unwrap_or_default();
        lint_yaml_fields(&name, &yaml_content, &mut msgs, &mut failed);
        lint_requires_plugins(&name, &yaml_content, &mut msgs, &mut failed);
        lint_requires_agents(&name, &yaml_content, &mut msgs, &mut failed);
    }

    if md_path.is_file() {
        msgs.push(format!("[PASS] {name}: SKILL.md exists"));
        let byte_size = std::fs::metadata(&md_path).map(|m| m.len()).unwrap_or(0);
        if byte_size <= TOKEN_BUDGET_BYTES {
            msgs.push(format!("[PASS] {name}: token budget ({byte_size}/{TOKEN_BUDGET_BYTES} bytes)"));
        } else {
            msgs.push(format!("[FAIL] {name}: SKILL.md over token budget ({byte_size}/{TOKEN_BUDGET_BYTES} bytes)"));
            failed = true;
        }
    } else {
        msgs.push(format!("[FAIL] {name}: SKILL.md missing"));
        failed = true;
    }

    LintResult { skill: name, ok: !failed, messages: msgs }
}

fn lint_yaml_fields(name: &str, yaml: &str, msgs: &mut Vec<String>, failed: &mut bool) {
    let missing: Vec<&str> = REQUIRED_FIELDS.iter()
        .filter(|&&f| yaml_get(yaml, f).is_none()).copied().collect();
    if missing.is_empty() {
        msgs.push(format!("[PASS] {name}: required fields present"));
    } else {
        msgs.push(format!("[FAIL] {name}: required fields missing: {}", missing.join(", ")));
        *failed = true;
    }
    match yaml_get(yaml, "constitution-version") {
        None => { msgs.push(format!("[FAIL] {name}: constitution-version not set")); *failed = true; }
        Some(ver) => {
            if semver_ge(&ver, MIN_CONSTITUTION_VERSION) {
                msgs.push(format!("[PASS] {name}: constitution version {ver} >= {MIN_CONSTITUTION_VERSION}"));
            } else {
                msgs.push(format!("[FAIL] {name}: constitution version {ver} < {MIN_CONSTITUTION_VERSION}"));
                *failed = true;
            }
        }
    }
    match yaml_get(yaml, "copyright") {
        Some(_) => msgs.push(format!("[PASS] {name}: copyright present")),
        None => { msgs.push(format!("[FAIL] {name}: copyright field missing or empty")); *failed = true; }
    }
    match yaml_get(yaml, "name") {
        Some(n) if name_format_valid(&n) => msgs.push(format!("[PASS] {name}: name format valid ({n})")),
        Some(n) => { msgs.push(format!("[FAIL] {name}: name format invalid ({n}), must match ^[a-z][a-z0-9-]*$")); *failed = true; }
        None => { msgs.push(format!("[FAIL] {name}: name field missing")); *failed = true; }
    }
    match yaml_get(yaml, "version") {
        Some(v) if version_format_valid(&v) => msgs.push(format!("[PASS] {name}: version format valid ({v})")),
        Some(v) => { msgs.push(format!("[FAIL] {name}: version format invalid ({v}), must be semver")); *failed = true; }
        None => { msgs.push(format!("[FAIL] {name}: version field missing")); *failed = true; }
    }
}

/// Validate requires-plugins: if present, must be non-empty list of strings.
fn lint_requires_plugins(name: &str, yaml: &str, msgs: &mut Vec<String>, failed: &mut bool) {
    if let Some(plugins) = yaml_get_list(yaml, "requires-plugins") {
        if plugins.is_empty() {
            msgs.push(format!("[FAIL] {name}: requires-plugins is empty (remove field or add entries)"));
            *failed = true;
        } else {
            msgs.push(format!("[PASS] {name}: requires-plugins valid ({} entries)", plugins.len()));
        }
    }
}

/// Validate requires-agents: each must match ^[a-z][a-z0-9-]*$.
fn lint_requires_agents(name: &str, yaml: &str, msgs: &mut Vec<String>, failed: &mut bool) {
    if let Some(agents) = yaml_get_list(yaml, "requires-agents") {
        if agents.is_empty() {
            msgs.push(format!("[FAIL] {name}: requires-agents is empty (remove field or add entries)"));
            *failed = true;
        } else {
            let invalid: Vec<&String> = agents.iter().filter(|a| !name_format_valid(a)).collect();
            if invalid.is_empty() {
                msgs.push(format!("[PASS] {name}: requires-agents valid ({} entries)", agents.len()));
            } else {
                let bad = invalid.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
                msgs.push(format!("[FAIL] {name}: requires-agents invalid names: {bad} (must match ^[a-z][a-z0-9-]*$)"));
                *failed = true;
            }
        }
    }
}

#[cfg(test)]
#[path = "cli_skill_tests.rs"]
mod tests;
