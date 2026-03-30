// Generates per-worktree .claude/settings.json as a replacement for
// --dangerously-skip-permissions. Task-scoped allow rules limit what
// commands claude can run inside each worktree.

/// Generate `.claude/settings.json` content for the given task type.
///
/// Returns a compact JSON string with `permissions.allow` rules scoped to the
/// task type. Write this to `{worktree}/.claude/settings.json` before
/// launching claude so it runs under least-privilege instead of
/// --dangerously-skip-permissions.
pub fn generate_worktree_settings(task_type: &str) -> String {
    let rules: &[&str] = match task_type {
        "rust" | "daemon" => &[
            "Bash(cargo check:*)",
            "Bash(cargo build:*)",
            "Bash(cargo test:*)",
            "Bash(cargo fmt:*)",
            "Bash(cargo clippy:*)",
            "Bash(git add:*)",
            "Bash(git commit:*)",
            "Bash(git diff:*)",
            "Bash(git status:*)",
            "Bash(git log:*)",
            "Bash(curl http://localhost:*)",
            "Bash(curl http://127.0.0.1:*)",
        ],
        "typescript" | "ts" | "evolution" => &[
            "Bash(npx tsc:*)",
            "Bash(npx vitest:*)",
            "Bash(npm run:*)",
            "Bash(npm install:*)",
            "Bash(git add:*)",
            "Bash(git commit:*)",
            "Bash(git diff:*)",
            "Bash(git status:*)",
            "Bash(git log:*)",
            "Bash(curl http://localhost:*)",
            "Bash(curl http://127.0.0.1:*)",
        ],
        "bash" | "scripts" => &[
            "Bash(bash:*)",
            "Bash(sh:*)",
            "Bash(git add:*)",
            "Bash(git commit:*)",
            "Bash(git diff:*)",
            "Bash(git status:*)",
            "Bash(git log:*)",
            "Bash(curl http://localhost:*)",
            "Bash(curl http://127.0.0.1:*)",
        ],
        // Default: minimal safe set covering most read/verify workflows
        _ => &[
            "Bash(cargo check:*)",
            "Bash(git add:*)",
            "Bash(git commit:*)",
            "Bash(git diff:*)",
            "Bash(git status:*)",
            "Bash(git log:*)",
            "Bash(curl http://localhost:*)",
            "Bash(curl http://127.0.0.1:*)",
        ],
    };

    serde_json::json!({
        "permissions": { "allow": rules }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_settings_no_dangerous_flag() {
        let s = generate_worktree_settings("rust");
        assert!(!s.contains("dangerously"));
        assert!(s.contains("cargo check"));
    }

    #[test]
    fn output_is_valid_json() {
        for task_type in &["rust", "typescript", "bash", "unknown"] {
            let s = generate_worktree_settings(task_type);
            serde_json::from_str::<serde_json::Value>(&s)
                .unwrap_or_else(|e| panic!("invalid JSON for {task_type}: {e}"));
        }
    }

    #[test]
    fn typescript_has_npm_rules() {
        let s = generate_worktree_settings("typescript");
        assert!(s.contains("npm run"));
        assert!(!s.contains("dangerously"));
    }

    #[test]
    fn default_type_is_safe() {
        let s = generate_worktree_settings("unknown-task");
        assert!(s.contains("git commit"));
        assert!(!s.contains("dangerously"));
    }
}
