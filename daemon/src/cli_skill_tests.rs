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

#[test]
fn skill_commands_lint_variant_exists() {
    use std::path::PathBuf;
    let cmd = SkillCommands::Lint {
        skill_dir: PathBuf::from("/tmp/skill"),
        human: false,
        all: false,
    };
    assert!(matches!(cmd, SkillCommands::Lint { .. }));
}

#[test]
fn skill_commands_transpile_variant_exists() {
    use std::path::PathBuf;
    let cmd = SkillCommands::Transpile {
        skill_dir: PathBuf::from("/tmp/skill"),
        output_dir: PathBuf::from("/tmp/out"),
        provider: "claude-code".into(),
        human: false,
    };
    assert!(matches!(cmd, SkillCommands::Transpile { .. }));
}
