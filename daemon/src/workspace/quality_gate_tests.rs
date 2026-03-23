// Tests for workspace::quality_gate — gate checks and serialisation.
use super::*;
use std::fs;
use tempfile::TempDir;

fn make_tmp() -> TempDir {
    tempfile::tempdir().expect("tempdir")
}

// ── check_clean_tree ──────────────────────────────────────────────────────────

#[test]
fn clean_tree_fails_gracefully_on_non_git_dir() {
    // A plain temp dir is not a git repo — git status exits non-zero and we get
    // passed=false with a descriptive message.
    let dir = make_tmp();
    let result = QualityGate::check_clean_tree(dir.path());
    assert_eq!(result.gate_name, "clean_tree");
    // Either passed (unlikely) or failed with a message — we mainly care that
    // the function doesn't panic and returns a structured GateResult.
    assert!(!result.message.is_empty());
}

#[test]
fn clean_tree_result_has_correct_gate_name() {
    let dir = make_tmp();
    let result = QualityGate::check_clean_tree(dir.path());
    assert_eq!(result.gate_name, "clean_tree");
}

#[test]
fn clean_tree_duration_is_non_negative() {
    let dir = make_tmp();
    let result = QualityGate::check_clean_tree(dir.path());
    // duration_ms is u64 — always >= 0
    let _ = result.duration_ms;
}

// ── check_file_sizes ─────────────────────────────────────────────────────────

#[test]
fn file_sizes_passes_when_dir_is_empty() {
    let dir = make_tmp();
    let result = QualityGate::check_file_sizes(dir.path(), 250);
    assert_eq!(result.gate_name, "file_sizes");
    assert!(result.passed, "empty dir should pass: {}", result.message);
}

#[test]
fn file_sizes_passes_when_file_under_limit() {
    let dir = make_tmp();
    fs::write(dir.path().join("small.rs"), "fn main() {}\n").unwrap();
    let result = QualityGate::check_file_sizes(dir.path(), 250);
    assert!(result.passed, "1-line file should pass");
}

#[test]
fn file_sizes_fails_when_file_exceeds_limit() {
    let dir = make_tmp();
    // Write a file with 251 lines
    let content = (0..251)
        .map(|i| format!("// line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(dir.path().join("big.rs"), content).unwrap();
    let result = QualityGate::check_file_sizes(dir.path(), 250);
    assert!(!result.passed, "251-line file should fail");
    assert!(
        result.message.contains("big.rs"),
        "message should name the file: {}",
        result.message
    );
}

#[test]
fn file_sizes_ignores_non_rs_files() {
    let dir = make_tmp();
    // A 1000-line .toml file should not trigger the gate
    let content = (0..1000)
        .map(|i| format!("key{i} = {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(dir.path().join("big.toml"), content).unwrap();
    let result = QualityGate::check_file_sizes(dir.path(), 250);
    assert!(result.passed, "non-.rs file should not count");
}

#[test]
fn file_sizes_recurses_into_subdirs() {
    let dir = make_tmp();
    let sub = dir.path().join("src");
    fs::create_dir(&sub).unwrap();
    let content = (0..300)
        .map(|i| format!("// line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(sub.join("deep.rs"), content).unwrap();
    let result = QualityGate::check_file_sizes(dir.path(), 250);
    assert!(!result.passed, "oversized nested file should fail");
}

// ── check_cargo ──────────────────────────────────────────────────────────────

#[test]
fn cargo_check_skips_when_no_cargo_toml() {
    let dir = make_tmp();
    let result = QualityGate::check_cargo(dir.path());
    assert_eq!(result.gate_name, "cargo_check");
    assert!(result.passed, "no Cargo.toml -> skip -> pass");
    assert!(result.message.contains("skipping"));
}

// ── check_tests ──────────────────────────────────────────────────────────────

#[test]
fn tests_check_skips_when_no_cargo_toml() {
    let dir = make_tmp();
    let result = QualityGate::check_tests(dir.path());
    assert_eq!(result.gate_name, "tests");
    assert!(result.passed, "no Cargo.toml -> skip -> pass");
    assert!(result.message.contains("skipping"));
}

// ── run_all ──────────────────────────────────────────────────────────────────

#[test]
fn run_all_returns_four_gates() {
    let dir = make_tmp();
    let results = QualityGate::run_all(dir.path());
    assert_eq!(results.len(), 4);
}

#[test]
fn run_all_gate_names_are_correct() {
    let dir = make_tmp();
    let results = QualityGate::run_all(dir.path());
    let names: Vec<&str> = results.iter().map(|r| r.gate_name.as_str()).collect();
    assert!(names.contains(&"clean_tree"));
    assert!(names.contains(&"file_sizes"));
    assert!(names.contains(&"cargo_check"));
    assert!(names.contains(&"tests"));
}

// ── serialisation ─────────────────────────────────────────────────────────────

#[test]
fn gate_result_serializes_to_json() {
    let r = GateResult {
        gate_name: "clean_tree".into(),
        passed: true,
        message: "ok".into(),
        duration_ms: 5,
    };
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"gate_name\""));
    assert!(json.contains("\"passed\""));
    assert!(json.contains("\"duration_ms\""));
}
