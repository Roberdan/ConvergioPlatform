use crate::hooks::checks::{CheckContext, CheckOutcome, DispatchState, HookCommand};
use crate::message_error::MessageResult;
use std::path::{Path, PathBuf};

pub fn contains_any(command: &str, values: &[&str]) -> bool {
    values.iter().any(|value| command.contains(value))
}

fn normalize_path(base: &Path, candidate: &str) -> PathBuf {
    let joined = if Path::new(candidate).is_absolute() {
        PathBuf::from(candidate)
    } else {
        base.join(candidate)
    };
    joined.canonicalize().unwrap_or(joined)
}

// Preferred: workspace operations should use /api/workspace/* endpoints (Rust workspace layer).
// worktree-create.sh, wave-worktree.sh, and pr-ops.sh are REMOVED. Use cvg workspace / cvg worktree.
pub fn check_worktree_guard(
    command: &HookCommand,
    context: &CheckContext,
    _state: &mut DispatchState,
) -> MessageResult<CheckOutcome> {
    if command.command.contains("git worktree add") {
        if let Some(path) = super::checks_support::extract_worktree_add_path(&command.command) {
            let resolved = normalize_path(&context.cwd, &path);
            let repo_root = context
                .repo_root
                .clone()
                .unwrap_or_else(|| context.cwd.clone());
            if resolved.starts_with(repo_root) {
                return Ok(CheckOutcome::Deny(
                    "WORKTREE GUARD: Path is INSIDE the repo. Use a SIBLING path instead."
                        .to_string(),
                ));
            }
        }
    }
    if command.command.contains("git worktree remove") {
        return Ok(CheckOutcome::Deny(
            "Use worktree-cleanup.sh instead of direct git worktree remove.".to_string(),
        ));
    }
    if contains_any(&command.command, &["git checkout -b", "git switch -c"])
        || (command.command.contains("git branch ")
            && !contains_any(
                &command.command,
                &[
                    "git branch -d",
                    "git branch -D",
                    "git branch --list",
                    "git branch --show",
                    "git branch --merged",
                    "git branch --no-merged",
                    "git branch --contains",
                ],
            ))
    {
        return Ok(CheckOutcome::Deny("BLOCKED: Never create bare branches. Use cvg workspace create-feature instead. See worktree-discipline.md § No Bare Branches.".to_string()));
    }
    if contains_any(
        &command.command,
        &[
            "git commit",
            "git push",
            "git add",
            "git checkout",
            "git merge",
            "git rebase",
            "git reset",
            "git stash",
        ],
    ) && !context.allow_main_write
        && matches!(context.current_branch.as_deref(), Some("main" | "master"))
    {
        return Ok(CheckOutcome::Deny(
            "BLOCKED: Git write on main/master is forbidden. Work in a worktree.".to_string(),
        ));
    }
    Ok(CheckOutcome::Continue)
}

pub fn check_warn_bash_antipatterns(
    command: &HookCommand,
    _context: &CheckContext,
    state: &mut DispatchState,
) -> MessageResult<CheckOutcome> {
    if command.command.contains("sqlite3")
        && command.command.contains("!=")
        && command.command.contains('"')
    {
        return Ok(CheckOutcome::Block(
            "BLOCKED: '!=' inside double-quoted sqlite3 command will break in zsh (! expansion)."
                .to_string(),
        ));
    }
    if command.command.contains(" find ") || command.command.starts_with("find ") {
        state
            .notices
            .push("ANTIPATTERN: Use Glob tool instead of bash".to_string());
    }
    if command.command.contains(" grep ")
        || command.command.starts_with("grep ")
        || command.command.starts_with("rg ")
    {
        state
            .notices
            .push("ANTIPATTERN: Use Grep tool instead of bash".to_string());
    }
    Ok(CheckOutcome::Continue)
}
