// Tests for mechanical validation gates.

use super::mechanical_gates::*;
use std::io::Write as IoWrite;

fn tmp(content: &str, suffix: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(suffix).tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

// -- credential scan --
#[test]
fn credential_scan_detects_api_key() {
    let f = tmp("let k = \"sk-abcdefghijklmnopqrstuvwxyz1234\";", ".rs");
    let r = gate_credential_scan(&[f.path().to_str().unwrap()]);
    assert!(!r.passed);
    assert!(r.details[0].contains("API key"));
}
#[test]
fn credential_scan_skips_test_file() {
    let f = tmp("let k = \"sk-abcdefghijklmnopqrstuvwxyz1234\";", "_test.rs");
    assert!(gate_credential_scan(&[f.path().to_str().unwrap()]).passed);
}
#[test]
fn credential_scan_detects_aws_key() {
    let f = tmp("aws = \"AKIAIOSFODNN7EXAMPLE\"", ".rs");
    assert!(!gate_credential_scan(&[f.path().to_str().unwrap()]).passed);
}
#[test]
fn credential_scan_detects_private_key() {
    let f = tmp("-----BEGIN RSA PRIVATE KEY-----", ".rs");
    assert!(!gate_credential_scan(&[f.path().to_str().unwrap()]).passed);
}
#[test]
fn credential_scan_detects_password() {
    let f = tmp("password = \"hunter2\"", ".rs");
    assert!(!gate_credential_scan(&[f.path().to_str().unwrap()]).passed);
}

// -- line count --
#[test]
fn line_count_rejects_long_file() {
    let f = tmp(&"x\n".repeat(300), ".rs");
    let r = gate_line_count(&[f.path().to_str().unwrap()], 250);
    assert!(!r.passed);
    assert!(r.details[0].contains("300"));
}
#[test]
fn line_count_accepts_short_file() {
    let f = tmp(&"x\n".repeat(100), ".rs");
    assert!(gate_line_count(&[f.path().to_str().unwrap()], 250).passed);
}

// -- pattern check --
#[test]
fn pattern_check_detects_todo_macro() {
    let f = tmp("fn main() { todo!() }", ".rs");
    assert!(!gate_pattern_check(&[f.path().to_str().unwrap()]).passed);
}
#[test]
fn pattern_check_detects_bare_todo() {
    let f = tmp("// TODO fix this later", ".rs");
    let r = gate_pattern_check(&[f.path().to_str().unwrap()]);
    assert!(!r.passed);
    assert!(r.details.iter().any(|d| d.contains("TODO without")));
}
#[test]
fn pattern_check_allows_todo_with_ref() {
    let f = tmp("// TODO(#123) tracked issue", ".rs");
    assert!(gate_pattern_check(&[f.path().to_str().unwrap()]).passed);
}
#[test]
fn pattern_check_detects_empty_catch() {
    let f = tmp("try { x() } catch (e) {}", ".ts");
    assert!(!gate_pattern_check(&[f.path().to_str().unwrap()]).passed);
}

// -- verify commands --
#[test]
fn verify_commands_pass() { assert!(gate_verify_commands(&["true", "echo ok"]).passed); }
#[test]
fn verify_commands_fail() {
    let r = gate_verify_commands(&["true", "false"]);
    assert!(!r.passed);
    assert!(r.details.iter().any(|d| d.starts_with("FAIL")));
}

// -- status + criteria --
#[test]
fn status_gate_passes_submitted() {
    let r = gate_status_check("submitted");
    assert!(r.passed);
    assert!(r.details.is_empty());
}
#[test]
fn status_gate_fails_non_submitted() {
    let r = gate_status_check("in_progress");
    assert!(!r.passed);
    assert!(r.details[0].contains("in_progress"));
}
#[test]
fn criteria_gate_passes_with_content() {
    assert!(gate_test_criteria(Some("[\"cargo test\"]")).passed);
}
#[test]
fn criteria_gate_fails_empty() {
    assert!(!gate_test_criteria(None).passed);
    assert!(!gate_test_criteria(Some("")).passed);
    assert!(!gate_test_criteria(Some("[]")).passed);
    assert!(!gate_test_criteria(Some("null")).passed);
}

// -- integration --
#[test]
fn validate_task_approved_for_valid_input() {
    let result = validate_task("submitted", Some("[\"t\"]"), &[], &[]);
    assert_eq!(result.status, "APPROVED");
    assert!(result.all_passed());
    assert!(!result.thor_invoked);
    assert_eq!(result.phase, "mechanical");
    assert!(result.note.contains("Thor validation"));
}
#[test]
fn validate_task_rejected_for_bad_status() {
    let result = validate_task("pending", Some("[\"t\"]"), &[], &[]);
    assert_eq!(result.status, "REJECTED");
    assert!(!result.all_passed());
    assert!(result.note.contains("fix issues"));
}
#[test]
fn validate_task_rejected_for_no_criteria() {
    assert_eq!(validate_task("submitted", None, &[], &[]).status, "REJECTED");
}
#[test]
fn run_all_gates_no_short_circuit() {
    let f = tmp("sk-abcdefghijklmnopqrstuvwxyz1234\ntodo!()", ".rs");
    let results = run_all_gates(&[f.path().to_str().unwrap()], &["false"]);
    assert_eq!(results.len(), 4, "all 4 gates must run");
    assert!(!results[0].passed, "credential gate");
    assert!(!results[2].passed, "pattern gate");
    assert!(!results[3].passed, "verify gate");
}

// -- summarize --
#[test]
fn summarize_all_pass() {
    let gates = vec![
        GateResult { gate: "a".into(), passed: true, details: vec![] },
        GateResult { gate: "b".into(), passed: true, details: vec![] },
    ];
    let s = summarize(gates);
    assert_eq!(s.status, "APPROVED");
    assert!(s.all_passed());
}
#[test]
fn summarize_one_fail() {
    let gates = vec![
        GateResult { gate: "a".into(), passed: true, details: vec![] },
        GateResult { gate: "b".into(), passed: false, details: vec!["bad".into()] },
    ];
    let s = summarize(gates);
    assert_eq!(s.status, "REJECTED");
    assert!(!s.all_passed());
}
