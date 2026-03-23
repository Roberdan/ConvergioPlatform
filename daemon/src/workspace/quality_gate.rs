// Quality gate checks — replaces pre-merge-gate.sh mechanical checks.
// Why: Plan 698 workspace layer; agents need programmatic merge readiness checks.
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_name: String,
    pub passed: bool,
    pub message: String,
    pub duration_ms: u64,
}

pub struct QualityGate;

impl QualityGate {
    /// Check working tree is clean (no unstaged changes).
    pub fn check_clean_tree(workspace_path: &Path) -> GateResult {
        let start = Instant::now();
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(workspace_path)
            .output();
        match output {
            Ok(o) if o.stdout.is_empty() => GateResult {
                gate_name: "clean_tree".into(),
                passed: true,
                message: "Working tree is clean".into(),
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Ok(o) => GateResult {
                gate_name: "clean_tree".into(),
                passed: false,
                message: format!(
                    "Uncommitted changes:\n{}",
                    String::from_utf8_lossy(&o.stdout)
                ),
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => GateResult {
                gate_name: "clean_tree".into(),
                passed: false,
                message: format!("git status failed: {e}"),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    /// Check all .rs files are under max_lines (default 250).
    pub fn check_file_sizes(workspace_path: &Path, max_lines: usize) -> GateResult {
        let start = Instant::now();
        let violations = collect_oversize_files(workspace_path, max_lines);
        GateResult {
            gate_name: "file_sizes".into(),
            passed: violations.is_empty(),
            message: if violations.is_empty() {
                format!("All files under {max_lines} lines")
            } else {
                format!(
                    "Files exceeding {max_lines} lines:\n{}",
                    violations.join("\n")
                )
            },
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Run cargo check (if Cargo.toml exists).
    pub fn check_cargo(workspace_path: &Path) -> GateResult {
        let start = Instant::now();
        if !workspace_path.join("Cargo.toml").exists() {
            return GateResult {
                gate_name: "cargo_check".into(),
                passed: true,
                message: "No Cargo.toml, skipping".into(),
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }
        let output = Command::new("cargo")
            .args(["check"])
            .current_dir(workspace_path)
            .output();
        match output {
            Ok(o) if o.status.success() => GateResult {
                gate_name: "cargo_check".into(),
                passed: true,
                message: "cargo check passed".into(),
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Ok(o) => GateResult {
                gate_name: "cargo_check".into(),
                passed: false,
                message: format!(
                    "cargo check failed:\n{}",
                    String::from_utf8_lossy(&o.stderr)
                ),
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => GateResult {
                gate_name: "cargo_check".into(),
                passed: false,
                message: format!("cargo check spawn failed: {e}"),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    /// Run cargo test (if Cargo.toml exists).
    pub fn check_tests(workspace_path: &Path) -> GateResult {
        let start = Instant::now();
        if !workspace_path.join("Cargo.toml").exists() {
            return GateResult {
                gate_name: "tests".into(),
                passed: true,
                message: "No Cargo.toml, skipping".into(),
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }
        let output = Command::new("cargo")
            .args(["test"])
            .current_dir(workspace_path)
            .output();
        match output {
            Ok(o) if o.status.success() => GateResult {
                gate_name: "tests".into(),
                passed: true,
                message: "All tests passed".into(),
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Ok(o) => GateResult {
                gate_name: "tests".into(),
                passed: false,
                message: format!("Tests failed:\n{}", String::from_utf8_lossy(&o.stderr)),
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => GateResult {
                gate_name: "tests".into(),
                passed: false,
                message: format!("cargo test spawn failed: {e}"),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    /// Run all gates and return results.
    pub fn run_all(workspace_path: &Path) -> Vec<GateResult> {
        vec![
            Self::check_clean_tree(workspace_path),
            Self::check_file_sizes(workspace_path, 250),
            Self::check_cargo(workspace_path),
            Self::check_tests(workspace_path),
        ]
    }
}

/// Recursively walk dir looking for .rs files over max_lines.
fn collect_oversize_files(dir: &Path, max_lines: usize) -> Vec<String> {
    let mut violations = Vec::new();
    walk_rs_files(dir, max_lines, &mut violations);
    violations
}

fn walk_rs_files(dir: &Path, max_lines: usize, violations: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_rs_files(&path, max_lines, violations);
        } else if path.extension().is_some_and(|e| e == "rs") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let lines = content.lines().count();
                if lines > max_lines {
                    violations.push(format!("{}: {} lines", path.display(), lines));
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "quality_gate_tests.rs"]
mod tests;
