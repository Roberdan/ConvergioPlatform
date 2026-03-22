//! Mechanical validation gates -- deterministic checks without AI judgment.

use regex::Regex;
use serde::Serialize;
use std::fs;
use std::process::Command;

/// Result of a single mechanical gate check.
#[derive(Debug, Clone, Serialize)]
pub struct GateResult {
    pub gate: String,
    pub passed: bool,
    pub details: Vec<String>,
}

/// Aggregated result of all mechanical gates for API responses.
#[derive(Debug, Clone, Serialize)]
pub struct MechanicalValidation {
    pub status: String,
    pub phase: String,
    pub gates: Vec<GateResult>,
    pub thor_invoked: bool,
    pub note: String,
}

impl MechanicalValidation {
    pub fn all_passed(&self) -> bool {
        self.status == "APPROVED"
    }
}

/// Build a MechanicalValidation from gate results.
pub fn summarize(gates: Vec<GateResult>) -> MechanicalValidation {
    let all_passed = gates.iter().all(|g| g.passed);
    MechanicalValidation {
        status: if all_passed { "APPROVED" } else { "REJECTED" }.to_string(),
        phase: "mechanical".to_string(),
        gates,
        thor_invoked: false,
        note: if all_passed {
            "mechanical gates passed, Thor validation at wave level".to_string()
        } else {
            "mechanical gates failed — fix issues before resubmitting".to_string()
        },
    }
}

/// Scan files for hardcoded credentials via regex. Skips *_test.rs and *.test.ts.
pub fn gate_credential_scan(files: &[&str]) -> GateResult {
    let patterns: Vec<(Regex, &str)> = vec![
        (Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap(), "API key"),
        (Regex::new(r"AKIA[A-Z0-9]{16}").unwrap(), "AWS access key"),
        (Regex::new(r#"password\s*=\s*["'][^"']+"#).unwrap(), "hardcoded password"),
        (Regex::new(r"BEGIN.*PRIVATE KEY").unwrap(), "private key"),
        (Regex::new(r"ghp_[a-zA-Z0-9]{36}").unwrap(), "GitHub PAT"),
    ];
    let mut violations = Vec::new();
    for fp in files {
        if fp.ends_with("_test.rs") || fp.ends_with(".test.ts") {
            continue;
        }
        let Ok(content) = fs::read_to_string(fp) else { continue };
        for (re, label) in &patterns {
            if re.is_match(&content) {
                violations.push(format!("{fp}: {label} detected"));
            }
        }
    }
    GateResult {
        gate: "credential_scan".to_string(),
        passed: violations.is_empty(),
        details: violations,
    }
}

/// Check that no file exceeds the given line limit.
pub fn gate_line_count(files: &[&str], max_lines: usize) -> GateResult {
    let mut violations = Vec::new();
    for fp in files {
        let Ok(content) = fs::read_to_string(fp) else { continue };
        let count = content.lines().count();
        if count > max_lines {
            violations.push(format!("{fp}: {count} lines (max {max_lines})"));
        }
    }
    GateResult {
        gate: "line_count".to_string(),
        passed: violations.is_empty(),
        details: violations,
    }
}

/// Scan files for anti-patterns: todo!(), bare TODO, empty catch blocks.
pub fn gate_pattern_check(files: &[&str]) -> GateResult {
    let todo_macro = Regex::new(r"todo!\(\)").unwrap();
    let todo_line = Regex::new(r"//\s*TODO").unwrap();
    let todo_ref = Regex::new(r"//\s*TODO\s*[\[(#]").unwrap();
    let empty_catch = Regex::new(r"catch\s*\([^)]*\)\s*\{\s*\}").unwrap();
    let unimplemented = Regex::new(r"unimplemented!\(\)").unwrap();

    let mut violations = Vec::new();
    for fp in files {
        let Ok(content) = fs::read_to_string(fp) else { continue };
        let is_rs = fp.ends_with(".rs");
        let is_ts_js = fp.ends_with(".ts") || fp.ends_with(".js");
        if is_rs && todo_macro.is_match(&content) {
            violations.push(format!("{fp}: contains todo!()"));
        }
        if is_rs && unimplemented.is_match(&content) {
            violations.push(format!("{fp}: contains unimplemented!()"));
        }
        if is_rs || is_ts_js {
            for line in content.lines() {
                if todo_line.is_match(line) && !todo_ref.is_match(line) {
                    violations.push(format!("{fp}: TODO without issue reference"));
                    break;
                }
            }
        }
        if is_ts_js && empty_catch.is_match(&content) {
            violations.push(format!("{fp}: empty catch block"));
        }
    }
    GateResult {
        gate: "pattern_check".to_string(),
        passed: violations.is_empty(),
        details: violations,
    }
}

/// Run shell commands and verify they all exit 0.
pub fn gate_verify_commands(commands: &[&str]) -> GateResult {
    let mut details = Vec::new();
    let mut all_ok = true;
    for cmd in commands {
        match Command::new("sh").arg("-c").arg(cmd).output() {
            Ok(o) if o.status.success() => details.push(format!("PASS: {cmd}")),
            Ok(o) => {
                all_ok = false;
                let stderr = String::from_utf8_lossy(&o.stderr);
                details.push(format!("FAIL: {cmd} (exit {:?}) {}", o.status.code(), stderr.trim()));
            }
            Err(e) => {
                all_ok = false;
                details.push(format!("ERROR: {cmd} — {e}"));
            }
        }
    }
    GateResult { gate: "verify_commands".to_string(), passed: all_ok, details }
}

/// Check that task status is "submitted" (required for validation).
pub fn gate_status_check(status: &str) -> GateResult {
    let ok = status == "submitted";
    GateResult {
        gate: "status_check".to_string(),
        passed: ok,
        details: if ok { vec![] } else { vec![format!("status is '{status}', expected 'submitted'")] },
    }
}

/// Check that test criteria are defined and non-empty.
pub fn gate_test_criteria(criteria: Option<&str>) -> GateResult {
    let has = criteria.map(|c| !c.is_empty() && c != "null" && c != "[]" && c != "{}").unwrap_or(false);
    GateResult {
        gate: "test_criteria".to_string(),
        passed: has,
        details: if has { vec![] } else { vec!["test criteria missing or empty".to_string()] },
    }
}

/// Run file-based mechanical gates (no short-circuit -- all gates always run).
pub fn run_all_gates(files: &[&str], commands: &[&str]) -> Vec<GateResult> {
    vec![
        gate_credential_scan(files),
        gate_line_count(files, 250),
        gate_pattern_check(files),
        gate_verify_commands(commands),
    ]
}

/// Run full validation: status + criteria + file gates.
pub fn validate_task(
    status: &str, test_criteria: Option<&str>, files: &[&str], verify_commands: &[&str],
) -> MechanicalValidation {
    let mut gates = vec![gate_status_check(status), gate_test_criteria(test_criteria)];
    if !files.is_empty() {
        gates.push(gate_credential_scan(files));
        gates.push(gate_line_count(files, 250));
        gates.push(gate_pattern_check(files));
    }
    if !verify_commands.is_empty() {
        gates.push(gate_verify_commands(verify_commands));
    }
    summarize(gates)
}

