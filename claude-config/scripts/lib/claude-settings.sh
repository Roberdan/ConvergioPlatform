#!/bin/bash
# lib/claude-settings.sh — Write per-worktree .claude/settings.json.
# Replaces --dangerously-skip-permissions with task-scoped allow rules.
# Usage: generate_claude_settings [worktree_path] [task_type]

generate_claude_settings() {
    local worktree="${1:-$(pwd)}"
    local task_type="${2:-default}"
    local settings_dir="${worktree}/.claude"
    local settings_file="${settings_dir}/settings.json"

    mkdir -p "$settings_dir" || return 1

    case "$task_type" in
        rust|daemon)
            local allow='"Bash(cargo check:*)","Bash(cargo build:*)","Bash(cargo test:*)","Bash(cargo fmt:*)","Bash(cargo clippy:*)","Bash(git add:*)","Bash(git commit:*)","Bash(git diff:*)","Bash(git status:*)","Bash(git log:*)","Bash(curl http://localhost:*)","Bash(curl http://127.0.0.1:*)"'
            ;;
        typescript|ts|evolution)
            local allow='"Bash(npx tsc:*)","Bash(npx vitest:*)","Bash(npm run:*)","Bash(npm install:*)","Bash(git add:*)","Bash(git commit:*)","Bash(git diff:*)","Bash(git status:*)","Bash(git log:*)","Bash(curl http://localhost:*)","Bash(curl http://127.0.0.1:*)"'
            ;;
        bash|scripts)
            local allow='"Bash(bash:*)","Bash(sh:*)","Bash(git add:*)","Bash(git commit:*)","Bash(git diff:*)","Bash(git status:*)","Bash(git log:*)","Bash(curl http://localhost:*)","Bash(curl http://127.0.0.1:*)"'
            ;;
        *)
            local allow='"Bash(cargo check:*)","Bash(git add:*)","Bash(git commit:*)","Bash(git diff:*)","Bash(git status:*)","Bash(git log:*)","Bash(curl http://localhost:*)","Bash(curl http://127.0.0.1:*)"'
            ;;
    esac

    printf '{"permissions":{"allow":[%s]}}\n' "$allow" > "$settings_file"
    echo "[claude-settings] wrote ${settings_file} (type=${task_type})" >&2
}
