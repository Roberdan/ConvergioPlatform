// Nightly evaluation tasks: git log analysis, token audit, memory audit, test health, deps.
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use tracing::warn;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub commits_today: usize,
    pub fix_chains: Vec<String>,
    pub failed_tests_in_log: usize,
    pub agents_over_limit: Vec<AgentLineCount>,
    pub stale_memory_files: Vec<String>,
    pub test_health: TestHealth,
    pub outdated_deps: Vec<OutdatedDep>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AgentLineCount {
    pub path: String,
    pub lines: usize,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TestHealth {
    pub passed: bool,
    pub output: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct OutdatedDep {
    pub name: String,
    pub current: String,
    pub latest: String,
}

/// Analyse recent git log for commit activity, failures, and fix-chains.
pub fn analyse_git_log(platform_dir: &Path) -> (usize, Vec<String>, usize) {
    let out = Command::new("git")
        .args(["log", "--oneline", "--since=24 hours ago"])
        .current_dir(platform_dir)
        .output();

    let Ok(out) = out else {
        return (0, vec![], 0);
    };
    let log = String::from_utf8_lossy(&out.stdout);

    let commits_today = log.lines().count();
    let mut fix_chains = Vec::new();
    let mut failed = 0usize;

    for line in log.lines() {
        let lower = line.to_lowercase();
        if lower.contains("fix") || lower.contains("revert") || lower.contains("hotfix") {
            fix_chains.push(line.to_string());
        }
        if lower.contains("fail") || lower.contains("error") || lower.contains("broken") {
            failed += 1;
        }
    }
    (commits_today, fix_chains, failed)
}

/// Count lines in agent definition files; flag files over 200 lines.
pub fn audit_agent_tokens(platform_dir: &Path) -> Vec<AgentLineCount> {
    let agents_dir = platform_dir.join(".github/agents");
    let Ok(entries) = std::fs::read_dir(&agents_dir) else {
        return vec![];
    };

    entries
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "md")
                .unwrap_or(false)
        })
        .filter_map(|e| {
            let content = std::fs::read_to_string(e.path()).ok()?;
            let lines = content.lines().count();
            if lines > 200 {
                Some(AgentLineCount {
                    path: e
                        .path()
                        .strip_prefix(platform_dir)
                        .unwrap_or(&e.path())
                        .to_string_lossy()
                        .to_string(),
                    lines,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Flag memory files not modified in 30 days.
pub fn audit_stale_memory(platform_dir: &Path) -> Vec<String> {
    let memory_dir = platform_dir.join(".claude/projects");
    let Ok(entries) = std::fs::read_dir(&memory_dir) else {
        return vec![];
    };

    let mut stale = Vec::new();
    for entry in entries.flatten() {
        let mem_path = entry.path().join("memory");
        if let Ok(mem_entries) = std::fs::read_dir(&mem_path) {
            for mf in mem_entries.flatten() {
                let path = mf.path();
                let is_stale = path
                    .metadata()
                    .and_then(|m| m.modified())
                    .map(|t| {
                        t.elapsed()
                            .map(|d| d.as_secs() > 30 * 86_400)
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);
                if is_stale {
                    stale.push(
                        path.strip_prefix(platform_dir)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .to_string(),
                    );
                }
            }
        }
    }
    stale
}

/// Run cargo test --lib; return pass/fail and trimmed output.
pub fn run_test_health(platform_dir: &Path) -> TestHealth {
    let out = Command::new("cargo")
        .args(["test", "--lib", "--quiet", "2>&1"])
        .current_dir(platform_dir.join("daemon"))
        .output();

    match out {
        Ok(o) => {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            let trimmed: String = combined.lines().take(30).collect::<Vec<_>>().join("\n");
            TestHealth {
                passed: o.status.success(),
                output: trimmed,
            }
        }
        Err(e) => TestHealth {
            passed: false,
            output: format!("cargo test failed to run: {e}"),
        },
    }
}

/// Check for outdated Cargo dependencies; parse `cargo outdated` output.
pub fn check_outdated_deps(platform_dir: &Path) -> Vec<OutdatedDep> {
    let out = Command::new("cargo")
        .args(["outdated", "--quiet"])
        .current_dir(platform_dir.join("daemon"))
        .output();

    let Ok(out) = out else {
        warn!("nightly: cargo outdated not available");
        return vec![];
    };

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut deps = Vec::new();

    // Parse table output: Name  Project  Compat  Latest  Kind  Platform
    for line in stdout.lines().skip(2) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 4 {
            deps.push(OutdatedDep {
                name: cols[0].to_string(),
                current: cols[1].to_string(),
                latest: cols[3].to_string(),
            });
        }
    }
    deps
}
