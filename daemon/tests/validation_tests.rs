use std::fs;
use tempfile::TempDir;

// Test that credential scan detects API keys
#[test]
fn test_credential_scan_detects_api_key() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("bad.rs");
    fs::write(&file, "let key = \"sk-1234567890abcdefghij\";").unwrap();
    let files = vec![file.to_str().unwrap()];
    let result = claude_core::validation::mechanical_gates::gate_credential_scan(&files);
    assert!(!result.passed, "Should detect API key");
}

#[test]
fn test_credential_scan_skips_test_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("my_test.rs");
    fs::write(&file, "let key = \"sk-1234567890abcdefghij\";").unwrap();
    let files = vec![file.to_str().unwrap()];
    let result = claude_core::validation::mechanical_gates::gate_credential_scan(&files);
    assert!(result.passed, "Should skip test files");
}

#[test]
fn test_line_count_rejects_long_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("long.rs");
    let content = "line\n".repeat(300);
    fs::write(&file, content).unwrap();
    let files = vec![file.to_str().unwrap()];
    let result = claude_core::validation::mechanical_gates::gate_line_count(&files, 250);
    assert!(!result.passed, "Should reject 300-line file");
}

#[test]
fn test_pattern_check_detects_todo_stub() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("stub.rs");
    fs::write(&file, "fn main() { todo!() }").unwrap();
    let files = vec![file.to_str().unwrap()];
    let result = claude_core::validation::mechanical_gates::gate_pattern_check(&files);
    assert!(!result.passed, "Should detect todo!()");
}

#[test]
fn test_verify_commands_pass() {
    let cmds = vec!["true", "test 1 -eq 1"];
    let result = claude_core::validation::mechanical_gates::gate_verify_commands(&cmds);
    assert!(result.passed);
}

#[test]
fn test_verify_commands_fail() {
    let cmds = vec!["false"];
    let result = claude_core::validation::mechanical_gates::gate_verify_commands(&cmds);
    assert!(!result.passed);
}

#[test]
fn test_run_all_gates_clean() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("clean.rs");
    fs::write(&file, "fn main() {}\n").unwrap();
    let files = vec![file.to_str().unwrap()];
    let cmds = vec!["true"];
    let results = claude_core::validation::mechanical_gates::run_all_gates(&files, &cmds);
    assert!(
        results.iter().all(|r| r.passed),
        "All gates should pass for clean file"
    );
}
