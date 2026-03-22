// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Skill lint+transpile subcommands — replaces skill-lint.sh and skill-transpile-*.sh.
use clap::Subcommand;
use std::path::PathBuf;

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
}

pub async fn handle(cmd: SkillCommands) {
    match cmd {
        SkillCommands::Lint { skill_dir, human, all } => {
            handle_lint(&skill_dir, human, all);
        }
        SkillCommands::Transpile { skill_dir, output_dir, provider, human } => {
            crate::cli_skill_transpile::handle_transpile(&skill_dir, &output_dir, &provider, human);
        }
    }
}

// ── Lint ──────────────────────────────────────────────────────────────────────

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
        let missing: Vec<&str> = REQUIRED_FIELDS.iter()
            .filter(|&&f| yaml_get(&yaml_content, f).is_none()).copied().collect();
        if missing.is_empty() {
            msgs.push(format!("[PASS] {name}: required fields present"));
        } else {
            msgs.push(format!("[FAIL] {name}: required fields missing: {}", missing.join(", ")));
            failed = true;
        }

        match yaml_get(&yaml_content, "constitution-version") {
            None => { msgs.push(format!("[FAIL] {name}: constitution-version not set")); failed = true; }
            Some(ver) => {
                if semver_ge(&ver, MIN_CONSTITUTION_VERSION) {
                    msgs.push(format!("[PASS] {name}: constitution version {ver} >= {MIN_CONSTITUTION_VERSION}"));
                } else {
                    msgs.push(format!("[FAIL] {name}: constitution version {ver} < {MIN_CONSTITUTION_VERSION}"));
                    failed = true;
                }
            }
        }

        match yaml_get(&yaml_content, "copyright") {
            Some(_) => msgs.push(format!("[PASS] {name}: copyright present")),
            None => { msgs.push(format!("[FAIL] {name}: copyright field missing or empty")); failed = true; }
        }

        match yaml_get(&yaml_content, "name") {
            Some(n) if name_format_valid(&n) => msgs.push(format!("[PASS] {name}: name format valid ({n})")),
            Some(n) => { msgs.push(format!("[FAIL] {name}: name format invalid ({n}), must match ^[a-z][a-z0-9-]*$")); failed = true; }
            None => { msgs.push(format!("[FAIL] {name}: name field missing")); failed = true; }
        }

        match yaml_get(&yaml_content, "version") {
            Some(v) if version_format_valid(&v) => msgs.push(format!("[PASS] {name}: version format valid ({v})")),
            Some(v) => { msgs.push(format!("[FAIL] {name}: version format invalid ({v}), must be semver")); failed = true; }
            None => { msgs.push(format!("[FAIL] {name}: version field missing")); failed = true; }
        }
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

// ── Helpers (pub(crate) for transpile module) ─────────────────────────────────
/// Extract a scalar YAML value (single-level key: value, unquoted or quoted).
pub(crate) fn yaml_get(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(&format!("{key}:")) {
            let val = rest.trim().trim_matches('"').trim_matches('\'').to_string();
            if !val.is_empty() { return Some(val); }
        }
    }
    None
}

/// Compare semver strings: returns true if `ver >= min`.
pub(crate) fn semver_ge(ver: &str, min: &str) -> bool {
    let parse = |s: &str| -> (u32, u32, u32) {
        let p: Vec<u32> = s.split('.').filter_map(|p| p.parse().ok()).collect();
        (p.first().copied().unwrap_or(0), p.get(1).copied().unwrap_or(0), p.get(2).copied().unwrap_or(0))
    };
    parse(ver) >= parse(min)
}

/// Validate skill name format: ^[a-z][a-z0-9-]*$
pub(crate) fn name_format_valid(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Validate semver format: ^[0-9]+\.[0-9]+\.[0-9]+$
pub(crate) fn version_format_valid(ver: &str) -> bool {
    let parts: Vec<&str> = ver.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// Strip a leading H1 line from Markdown body.
pub(crate) fn strip_h1(md: &str) -> String {
    let mut lines = md.lines();
    match lines.next() {
        Some(first) if first.starts_with("# ") => lines.collect::<Vec<_>>().join("\n"),
        Some(first) => {
            let rest: Vec<_> = lines.collect();
            if rest.is_empty() { first.to_string() } else { format!("{first}\n{}", rest.join("\n")) }
        }
        None => String::new(),
    }
}

/// Capitalise first character of a string.
pub(crate) fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
#[path = "cli_skill_tests.rs"]
mod tests;
