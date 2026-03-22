// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for cli_skill and cli_skill_transpile modules.

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
    fs::write(skill_dir.join("SKILL.md"), &"x".repeat(7000)).unwrap();
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

// Helper tests (yaml_get, semver_ge, name_format_valid, etc.) live in cli_skill_validate::tests

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
    let path = crate::cli_skill_transpile::transpile_claude(&yaml, &md, &out_dir).unwrap();
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
    let path = crate::cli_skill_transpile::transpile_copilot(&yaml, &md, &out_dir).unwrap();
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
    let path = crate::cli_skill_transpile::transpile_generic(&yaml, &md, &out_dir).unwrap();
    assert!(path.exists());
    assert!(path.to_string_lossy().ends_with(".system-prompt.txt"));
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("You are my-skill"));
    assert!(content.contains("## Instructions"));
}

fn make_skill_with_yaml(dir: &std::path::Path, extra_yaml: &str) {
    fs::write(dir.join("skill.yaml"), format!("\
name: test-skill\nversion: 1.0.0\ndescription: A skill\ndomain: testing\n\
constitution-version: 2.0.0\nlicense: MPL-2.0\ncopyright: Roberto D'Angelo, 2026\n{extra_yaml}")).unwrap();
    fs::write(dir.join("SKILL.md"), "# test-skill\n\nContent.\n").unwrap();
}

#[test]
fn lint_requires_plugins_valid_passes() {
    let tmp = TempDir::new().unwrap();
    let sd = tmp.path().join("plug-skill");
    fs::create_dir(&sd).unwrap();
    make_skill_with_yaml(&sd, "requires-plugins: [mcp-github, mcp-slack]\n");
    let result = lint_one(&sd);
    assert!(result.ok, "messages: {:?}", result.messages);
    assert!(result.messages.iter().any(|m| m.contains("requires-plugins valid")));
}

#[test]
fn lint_requires_agents_valid_passes() {
    let tmp = TempDir::new().unwrap();
    let sd = tmp.path().join("agent-skill");
    fs::create_dir(&sd).unwrap();
    make_skill_with_yaml(&sd, "requires-agents:\n  - my-agent\n  - another-agent\n");
    let result = lint_one(&sd);
    assert!(result.ok, "messages: {:?}", result.messages);
    assert!(result.messages.iter().any(|m| m.contains("requires-agents valid")));
}

#[test]
fn lint_requires_agents_invalid_name_fails() {
    let tmp = TempDir::new().unwrap();
    let sd = tmp.path().join("bad-agent-skill");
    fs::create_dir(&sd).unwrap();
    make_skill_with_yaml(&sd, "requires-agents:\n  - BadAgent\n  - ok-agent\n");
    let result = lint_one(&sd);
    assert!(!result.ok);
    assert!(result.messages.iter().any(|m| m.contains("requires-agents invalid")));
}

#[test]
fn lint_no_requires_fields_still_passes() {
    let tmp = TempDir::new().unwrap();
    let sd = tmp.path().join("no-req-skill");
    fs::create_dir(&sd).unwrap();
    make_skill_with_yaml(&sd, "");
    let result = lint_one(&sd);
    assert!(result.ok, "messages: {:?}", result.messages);
}

#[test]
fn skill_commands_enable_variant_exists() {
    let cmd = SkillCommands::Enable {
        skill_dir: PathBuf::from("/tmp/skill"),
        api_url: "http://localhost:8420".into(),
        human: false,
    };
    assert!(matches!(cmd, SkillCommands::Enable { .. }));
}
