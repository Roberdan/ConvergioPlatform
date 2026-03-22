// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Skill lint and transpile subcommands — replaces skill-lint.sh and skill-transpile-*.sh.
// JSON output by default; --human for readable text.

use clap::Subcommand;
use std::path::PathBuf;

const MIN_CONSTITUTION_VERSION: &str = "2.0.0";
const TOKEN_BUDGET_BYTES: u64 = 6144;
const REQUIRED_FIELDS: &[&str] = &[
    "name",
    "version",
    "description",
    "domain",
    "constitution-version",
    "license",
    "copyright",
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
            handle_transpile(&skill_dir, &output_dir, &provider, human);
        }
    }
}

// ── Lint ──────────────────────────────────────────────────────────────────────

fn handle_lint(skill_dir: &PathBuf, human: bool, all: bool) {
    if all {
        // Lint every subdirectory
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
                let result = lint_one(&entry.path());
                results.push(result);
            }
        }
        if results.is_empty() {
            eprintln!("no skill directories found in {}", skill_dir.display());
            std::process::exit(1);
        }
        let pass: usize = results.iter().filter(|r| r.ok).count();
        let fail: usize = results.iter().filter(|r| !r.ok).count();
        if human {
            for r in &results {
                for msg in &r.messages {
                    println!("{msg}");
                }
            }
            println!("\nSummary: {pass} passed, {fail} failed");
        } else {
            let json = serde_json::json!({
                "results": results.iter().map(|r| r.to_json()).collect::<Vec<_>>(),
                "summary": {"pass": pass, "fail": fail}
            });
            println!("{json}");
        }
        if fail > 0 {
            std::process::exit(1);
        }
    } else {
        let result = lint_one(skill_dir);
        let pass: usize = if result.ok { 1 } else { 0 };
        let fail: usize = 1 - pass;
        if human {
            for msg in &result.messages {
                println!("{msg}");
            }
            println!("\nSummary: {pass} passed, {fail} failed");
        } else {
            let json = serde_json::json!({
                "results": [result.to_json()],
                "summary": {"pass": pass, "fail": fail}
            });
            println!("{json}");
        }
        if !result.ok {
            std::process::exit(1);
        }
    }
}

struct LintResult {
    skill: String,
    ok: bool,
    messages: Vec<String>,
}

impl LintResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "skill": self.skill,
            "ok": self.ok,
            "messages": self.messages,
        })
    }
}

fn lint_one(skill_dir: &PathBuf) -> LintResult {
    let name = skill_dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let yaml_path = skill_dir.join("skill.yaml");
    let md_path = skill_dir.join("SKILL.md");
    let mut msgs: Vec<String> = Vec::new();
    let mut failed = false;

    // 1. skill.yaml exists
    if yaml_path.is_file() {
        msgs.push(format!("[PASS] {name}: skill.yaml exists"));
    } else {
        msgs.push(format!("[FAIL] {name}: skill.yaml missing"));
        failed = true;
    }

    // 2. Required fields (only if skill.yaml present)
    if yaml_path.is_file() {
        let yaml_content = std::fs::read_to_string(&yaml_path).unwrap_or_default();
        let missing: Vec<&str> = REQUIRED_FIELDS.iter()
            .filter(|&&f| yaml_get(&yaml_content, f).is_none())
            .copied()
            .collect();
        if missing.is_empty() {
            msgs.push(format!("[PASS] {name}: required fields present"));
        } else {
            msgs.push(format!("[FAIL] {name}: required fields missing: {}", missing.join(", ")));
            failed = true;
        }

        // 3. Constitution version
        match yaml_get(&yaml_content, "constitution-version") {
            None => {
                msgs.push(format!("[FAIL] {name}: constitution-version not set"));
                failed = true;
            }
            Some(ver) => {
                if semver_ge(&ver, MIN_CONSTITUTION_VERSION) {
                    msgs.push(format!("[PASS] {name}: constitution version {ver} >= {MIN_CONSTITUTION_VERSION}"));
                } else {
                    msgs.push(format!("[FAIL] {name}: constitution version {ver} < {MIN_CONSTITUTION_VERSION}"));
                    failed = true;
                }
            }
        }

        // 4. Copyright non-empty
        match yaml_get(&yaml_content, "copyright") {
            Some(_) => msgs.push(format!("[PASS] {name}: copyright present")),
            None => {
                msgs.push(format!("[FAIL] {name}: copyright field missing or empty"));
                failed = true;
            }
        }

        // 5. Name format ^[a-z][a-z0-9-]*$
        match yaml_get(&yaml_content, "name") {
            Some(n) if name_format_valid(&n) => {
                msgs.push(format!("[PASS] {name}: name format valid ({n})"));
            }
            Some(n) => {
                msgs.push(format!("[FAIL] {name}: name format invalid ({n}), must match ^[a-z][a-z0-9-]*$"));
                failed = true;
            }
            None => {
                msgs.push(format!("[FAIL] {name}: name field missing"));
                failed = true;
            }
        }

        // 6. Version format ^[0-9]+\.[0-9]+\.[0-9]+$
        match yaml_get(&yaml_content, "version") {
            Some(v) if version_format_valid(&v) => {
                msgs.push(format!("[PASS] {name}: version format valid ({v})"));
            }
            Some(v) => {
                msgs.push(format!("[FAIL] {name}: version format invalid ({v}), must be semver"));
                failed = true;
            }
            None => {
                msgs.push(format!("[FAIL] {name}: version field missing"));
                failed = true;
            }
        }
    }

    // 7. SKILL.md exists
    if md_path.is_file() {
        msgs.push(format!("[PASS] {name}: SKILL.md exists"));
    } else {
        msgs.push(format!("[FAIL] {name}: SKILL.md missing"));
        failed = true;
    }

    // 8. Token budget (only if SKILL.md present)
    if md_path.is_file() {
        let byte_size = std::fs::metadata(&md_path).map(|m| m.len()).unwrap_or(0);
        if byte_size <= TOKEN_BUDGET_BYTES {
            msgs.push(format!("[PASS] {name}: token budget ({byte_size}/{TOKEN_BUDGET_BYTES} bytes)"));
        } else {
            msgs.push(format!("[FAIL] {name}: SKILL.md over token budget ({byte_size}/{TOKEN_BUDGET_BYTES} bytes)"));
            failed = true;
        }
    }

    LintResult { skill: name, ok: !failed, messages: msgs }
}

// ── Transpile ─────────────────────────────────────────────────────────────────

fn handle_transpile(skill_dir: &PathBuf, output_dir: &PathBuf, provider: &str, human: bool) {
    let yaml_path = skill_dir.join("skill.yaml");
    let md_path = skill_dir.join("SKILL.md");

    for p in [&yaml_path, &md_path] {
        if !p.is_file() {
            eprintln!("missing: {}", p.display());
            std::process::exit(2);
        }
    }

    let yaml_content = match std::fs::read_to_string(&yaml_path) {
        Ok(c) => c,
        Err(e) => { eprintln!("read error: {e}"); std::process::exit(2); }
    };
    let md_content = match std::fs::read_to_string(&md_path) {
        Ok(c) => c,
        Err(e) => { eprintln!("read error: {e}"); std::process::exit(2); }
    };

    let output = match provider {
        "claude-code" => transpile_claude(&yaml_content, &md_content, skill_dir, output_dir),
        "copilot-cli" => transpile_copilot(&yaml_content, &md_content, skill_dir, output_dir),
        "generic-llm" => transpile_generic(&yaml_content, &md_content, skill_dir, output_dir),
        other => {
            eprintln!("unknown provider '{other}'; use: claude-code, copilot-cli, generic-llm");
            std::process::exit(2);
        }
    };

    match output {
        Ok(path) => {
            if human {
                println!("Written: {}", path.display());
            } else {
                println!("{}", serde_json::json!({"ok": true, "path": path.display().to_string()}));
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    }
}

fn transpile_claude(yaml: &str, md: &str, _skill_dir: &PathBuf, output_dir: &PathBuf) -> Result<PathBuf, String> {
    let name = yaml_get(yaml, "name").ok_or("skill.yaml missing 'name'")?;
    let version = yaml_get(yaml, "version").ok_or("skill.yaml missing 'version'")?;
    let description = yaml_get(yaml, "description").unwrap_or_default();
    let arguments = yaml_get(yaml, "arguments").unwrap_or_default();

    let md_body = strip_h1(md);
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("name: {name}\n"));
    out.push_str(&format!("version: \"{version}\"\n"));
    out.push_str("---\n\n");
    out.push_str(&format!("<!-- v{version} -->\n\n"));
    out.push_str(&format!("# {name}\n\n"));
    if !description.is_empty() {
        out.push_str(&format!("{description}\n\n"));
    }
    if !arguments.is_empty() && arguments != "none" {
        out.push_str(&format!("ARGUMENTS: {arguments}\n\n"));
    }
    out.push_str(&md_body);

    let out_path = output_dir.join(format!("{name}.md"));
    std::fs::write(&out_path, &out).map_err(|e| format!("write error: {e}"))?;
    Ok(out_path)
}

fn transpile_copilot(yaml: &str, md: &str, _skill_dir: &PathBuf, output_dir: &PathBuf) -> Result<PathBuf, String> {
    let name = yaml_get(yaml, "name").ok_or("skill.yaml missing 'name'")?;
    let version = yaml_get(yaml, "version").ok_or("skill.yaml missing 'version'")?;
    let description = yaml_get(yaml, "description").unwrap_or_default();
    let model = yaml_get(yaml, "model").unwrap_or_default();
    let _ = version; // included via body

    let md_body = strip_h1(md);
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("name: {name}\n"));
    if !description.is_empty() {
        out.push_str(&format!("description: {description}\n"));
    }
    if !model.is_empty() {
        out.push_str(&format!("model: {model}\n"));
    }
    out.push_str("---\n\n");
    let cap_name = capitalise(&name);
    out.push_str(&format!("# {cap_name}\n\n"));
    out.push_str(&md_body);

    let out_path = output_dir.join(format!("{name}.agent.md"));
    std::fs::write(&out_path, &out).map_err(|e| format!("write error: {e}"))?;
    Ok(out_path)
}

fn transpile_generic(yaml: &str, md: &str, _skill_dir: &PathBuf, output_dir: &PathBuf) -> Result<PathBuf, String> {
    let name = yaml_get(yaml, "name").ok_or("skill.yaml missing 'name'")?;
    let description = yaml_get(yaml, "description").unwrap_or_default();
    let domain = yaml_get(yaml, "domain").unwrap_or_else(|| "general".into());
    let const_ver = yaml_get(yaml, "constitution-version").unwrap_or_else(|| "2.0.0".into());
    let license = yaml_get(yaml, "license").unwrap_or_else(|| "MPL-2.0".into());

    let mut out = String::new();
    out.push_str(&format!("You are {name}, a {description}.\n\n"));
    out.push_str(&format!("Domain: {domain}\n"));
    out.push_str(&format!("Constitution: v{const_ver}\n"));
    out.push_str(&format!("License: {license}\n\n"));
    out.push_str("## Instructions\n\n");
    out.push_str(md);
    out.push_str("\n\n## Constraints\n");
    out.push_str(&format!("- Follow the Convergio Constitution v{const_ver}\n"));
    out.push_str(&format!("- {license} licensed\n"));

    let safe_name: String = name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let out_path = output_dir.join(format!("{safe_name}.system-prompt.txt"));
    std::fs::write(&out_path, &out).map_err(|e| format!("write error: {e}"))?;
    Ok(out_path)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract a scalar YAML value (single-level key: value, unquoted or quoted).
fn yaml_get(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(&format!("{key}:")) {
            let val = rest.trim().trim_matches('"').trim_matches('\'').to_string();
            if !val.is_empty() {
                return Some(val);
            }
        }
    }
    None
}

/// Compare semver strings: returns true if `ver >= min`.
fn semver_ge(ver: &str, min: &str) -> bool {
    let parse = |s: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = s.split('.').filter_map(|p| p.parse().ok()).collect();
        (parts.first().copied().unwrap_or(0), parts.get(1).copied().unwrap_or(0), parts.get(2).copied().unwrap_or(0))
    };
    parse(ver) >= parse(min)
}

/// Validate skill name format: ^[a-z][a-z0-9-]*$
fn name_format_valid(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Validate semver format: ^[0-9]+\.[0-9]+\.[0-9]+$
fn version_format_valid(ver: &str) -> bool {
    let parts: Vec<&str> = ver.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// Strip a leading H1 line from Markdown body.
fn strip_h1(md: &str) -> String {
    let mut lines = md.lines();
    match lines.next() {
        Some(first) if first.starts_with("# ") => lines.collect::<Vec<_>>().join("\n"),
        Some(first) => {
            let rest: Vec<_> = lines.collect();
            if rest.is_empty() {
                first.to_string()
            } else {
                format!("{first}\n{}", rest.join("\n"))
            }
        }
        None => String::new(),
    }
}

/// Capitalise first character of a string.
fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_valid_skill(dir: &std::path::Path) {
        fs::write(dir.join("skill.yaml"), "\
name: my-skill\n\
version: 1.0.0\n\
description: A test skill\n\
domain: testing\n\
constitution-version: 2.0.0\n\
license: MPL-2.0\n\
copyright: Roberto D'Angelo, 2026\n").unwrap();
        fs::write(dir.join("SKILL.md"), "# my-skill\n\nDoes things.\n").unwrap();
    }

    #[test]
    fn lint_valid_skill_passes() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("my-skill");
        fs::create_dir(&skill_dir).unwrap();
        make_valid_skill(&skill_dir);
        let result = lint_one(&skill_dir);
        assert!(result.ok, "expected lint to pass; messages: {:?}", result.messages);
    }

    #[test]
    fn lint_missing_yaml_fails() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("bad-skill");
        fs::create_dir(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# bad-skill\n\nContent.\n").unwrap();
        let result = lint_one(&skill_dir);
        assert!(!result.ok);
        assert!(result.messages.iter().any(|m| m.contains("[FAIL]") && m.contains("skill.yaml missing")));
    }

    #[test]
    fn lint_missing_skill_md_fails() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("no-md");
        fs::create_dir(&skill_dir).unwrap();
        fs::write(skill_dir.join("skill.yaml"), "\
name: no-md\n\
version: 1.0.0\n\
description: A skill\n\
domain: testing\n\
constitution-version: 2.0.0\n\
license: MPL-2.0\n\
copyright: Roberto D'Angelo, 2026\n").unwrap();
        let result = lint_one(&skill_dir);
        assert!(!result.ok);
        assert!(result.messages.iter().any(|m| m.contains("[FAIL]") && m.contains("SKILL.md missing")));
    }

    #[test]
    fn lint_old_constitution_version_fails() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("old-skill");
        fs::create_dir(&skill_dir).unwrap();
        fs::write(skill_dir.join("skill.yaml"), "\
name: old-skill\n\
version: 1.0.0\n\
description: A skill\n\
domain: testing\n\
constitution-version: 1.0.0\n\
license: MPL-2.0\n\
copyright: Roberto D'Angelo, 2026\n").unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# old-skill\n\nContent.\n").unwrap();
        let result = lint_one(&skill_dir);
        assert!(!result.ok);
        assert!(result.messages.iter().any(|m| m.contains("[FAIL]") && m.contains("constitution version")));
    }

    #[test]
    fn lint_over_token_budget_fails() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("big-skill");
        fs::create_dir(&skill_dir).unwrap();
        fs::write(skill_dir.join("skill.yaml"), "\
name: big-skill\n\
version: 1.0.0\n\
description: A skill\n\
domain: testing\n\
constitution-version: 2.0.0\n\
license: MPL-2.0\n\
copyright: Roberto D'Angelo, 2026\n").unwrap();
        // Write >6144 bytes
        let big = "x".repeat(7000);
        fs::write(skill_dir.join("SKILL.md"), &big).unwrap();
        let result = lint_one(&skill_dir);
        assert!(!result.ok);
        assert!(result.messages.iter().any(|m| m.contains("[FAIL]") && m.contains("token budget")));
    }

    #[test]
    fn lint_invalid_name_format_fails() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("BadName");
        fs::create_dir(&skill_dir).unwrap();
        fs::write(skill_dir.join("skill.yaml"), "\
name: BadName\n\
version: 1.0.0\n\
description: A skill\n\
domain: testing\n\
constitution-version: 2.0.0\n\
license: MPL-2.0\n\
copyright: Roberto D'Angelo, 2026\n").unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# BadName\n\nContent.\n").unwrap();
        let result = lint_one(&skill_dir);
        assert!(!result.ok);
        assert!(result.messages.iter().any(|m| m.contains("[FAIL]") && m.contains("name format invalid")));
    }

    #[test]
    fn yaml_get_extracts_unquoted() {
        let content = "name: my-skill\nversion: 1.0.0\n";
        assert_eq!(yaml_get(content, "name"), Some("my-skill".into()));
        assert_eq!(yaml_get(content, "version"), Some("1.0.0".into()));
    }

    #[test]
    fn yaml_get_extracts_quoted() {
        let content = "version: \"2.3.0\"\ncopyright: 'Roberto D Angelo'\n";
        assert_eq!(yaml_get(content, "version"), Some("2.3.0".into()));
        assert_eq!(yaml_get(content, "copyright"), Some("Roberto D Angelo".into()));
    }

    #[test]
    fn yaml_get_missing_key_returns_none() {
        assert_eq!(yaml_get("name: foo\n", "missing"), None);
    }

    #[test]
    fn semver_ge_comparisons() {
        assert!(semver_ge("2.0.0", "2.0.0"));
        assert!(semver_ge("3.0.0", "2.0.0"));
        assert!(semver_ge("2.1.0", "2.0.0"));
        assert!(!semver_ge("1.9.9", "2.0.0"));
        assert!(!semver_ge("1.0.0", "2.0.0"));
    }

    #[test]
    fn name_format_valid_cases() {
        assert!(name_format_valid("my-skill"));
        assert!(name_format_valid("skill123"));
        assert!(!name_format_valid("MySkill"));
        assert!(!name_format_valid("-bad"));
        assert!(!name_format_valid(""));
    }

    #[test]
    fn version_format_valid_cases() {
        assert!(version_format_valid("1.0.0"));
        assert!(version_format_valid("10.22.333"));
        assert!(!version_format_valid("1.0"));
        assert!(!version_format_valid("v1.0.0"));
        assert!(!version_format_valid("1.0.0-beta"));
    }

    #[test]
    fn strip_h1_removes_title_line() {
        let md = "# my-skill\n\nDoes things.\n";
        let stripped = strip_h1(md);
        assert!(!stripped.contains("# my-skill"), "H1 should be stripped");
        assert!(stripped.contains("Does things."));
    }

    #[test]
    fn capitalise_first_char() {
        assert_eq!(capitalise("planner"), "Planner");
        assert_eq!(capitalise("my-skill"), "My-skill");
        assert_eq!(capitalise(""), "");
    }

    #[test]
    fn transpile_claude_creates_file() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("test-skill");
        fs::create_dir(&skill_dir).unwrap();
        make_valid_skill(&skill_dir);
        let yaml = fs::read_to_string(skill_dir.join("skill.yaml")).unwrap();
        let md = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        let out_dir = tmp.path().join("out");
        fs::create_dir(&out_dir).unwrap();
        let path = transpile_claude(&yaml, &md, &skill_dir, &out_dir).unwrap();
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("---\nname: my-skill"));
        assert!(content.contains("<!-- v1.0.0 -->"));
    }

    #[test]
    fn transpile_copilot_creates_agent_md() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("test-skill");
        fs::create_dir(&skill_dir).unwrap();
        make_valid_skill(&skill_dir);
        let yaml = fs::read_to_string(skill_dir.join("skill.yaml")).unwrap();
        let md = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        let out_dir = tmp.path().join("out");
        fs::create_dir(&out_dir).unwrap();
        let path = transpile_copilot(&yaml, &md, &skill_dir, &out_dir).unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().ends_with(".agent.md"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# My-skill"));
    }

    #[test]
    fn transpile_generic_creates_system_prompt() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("test-skill");
        fs::create_dir(&skill_dir).unwrap();
        make_valid_skill(&skill_dir);
        let yaml = fs::read_to_string(skill_dir.join("skill.yaml")).unwrap();
        let md = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        let out_dir = tmp.path().join("out");
        fs::create_dir(&out_dir).unwrap();
        let path = transpile_generic(&yaml, &md, &skill_dir, &out_dir).unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().ends_with(".system-prompt.txt"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("You are my-skill"));
        assert!(content.contains("## Instructions"));
    }

    #[test]
    fn skill_commands_lint_variant_exists() {
        let cmd = SkillCommands::Lint {
            skill_dir: PathBuf::from("/tmp/skill"),
            human: false,
            all: false,
        };
        assert!(matches!(cmd, SkillCommands::Lint { .. }));
    }

    #[test]
    fn skill_commands_transpile_variant_exists() {
        let cmd = SkillCommands::Transpile {
            skill_dir: PathBuf::from("/tmp/skill"),
            output_dir: PathBuf::from("/tmp/out"),
            provider: "claude-code".into(),
            human: false,
        };
        assert!(matches!(cmd, SkillCommands::Transpile { .. }));
    }
}
