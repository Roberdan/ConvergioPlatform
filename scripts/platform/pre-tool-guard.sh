#!/usr/bin/env bash
set -euo pipefail

# Consolidated PreToolUse guard — replaces 13 individual hooks.
# Usage: echo '{"command":"..."}' | pre-tool-guard.sh <tool>
# Exit 0 = allow, Exit 2 = blocked (message on stderr)

TOOL="${1:-}"
INPUT=$(cat 2>/dev/null || true)

block() {
  echo "BLOCKED: $1" >&2
  exit 2
}

warn() {
  echo "WARNING: $1" >&2
}

# Extract command for Bash tool
cmd() {
  echo "$INPUT" | jq -r '.command // empty' 2>/dev/null || echo "$INPUT"
}

# Extract file_path for Edit/Write tools
file_path() {
  echo "$INPUT" | jq -r '.file_path // empty' 2>/dev/null
}

# Extract new_string for Edit tool
new_string() {
  echo "$INPUT" | jq -r '.new_string // empty' 2>/dev/null
}

guard_bash() {
  local CMD
  CMD=$(cmd)
  [ -z "$CMD" ] && return 0

  # plan-db.sh update-task done — use plan-db-safe.sh
  if echo "$CMD" | grep -qE 'plan-db\.sh\s+update-task.*done'; then
    block "Use plan-db-safe.sh for marking tasks done"
  fi

  # plan-db.sh create/import — use planner-create.sh
  if echo "$CMD" | grep -qE 'plan-db\.sh\s+(create|import)'; then
    block "Use planner-create.sh instead of plan-db.sh create/import (enforces review gate)"
  fi

  # git checkout/switch/branch on main — use worktree-create.sh
  if echo "$CMD" | grep -qE 'git\s+(checkout|switch)' \
     || { echo "$CMD" | grep -qE 'git\s+branch' \
          && ! echo "$CMD" | grep -qE 'git\s+branch\s+(-d|-D|--delete|--list|-a|-r)'; }; then
    local BRANCH
    BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)
    if [ "$BRANCH" = "main" ] || [ "$BRANCH" = "master" ]; then
      block "Never create branches on main -- use worktree-create.sh"
    fi
  fi

  # npm run lint/test/build — use digest scripts
  if echo "$CMD" | grep -qE 'npm\s+run\s+(lint|test|build|typecheck)'; then
    block "Use digest scripts instead (test-digest.sh, build-digest.sh)"
  fi

  # gh run view --log — use ci-digest.sh
  if echo "$CMD" | grep -qE 'gh\s+run\s+view\s+--log'; then
    block "Use ci-digest.sh instead of gh run view --log"
  fi

  # gh pr view/checks — use pr-ops.sh or ci-digest.sh
  if echo "$CMD" | grep -qE 'gh\s+pr\s+(view|checks)'; then
    block "Use pr-ops.sh or ci-digest.sh checks"
  fi

  # UPDATE waves SET status='done' — requires Thor
  if echo "$CMD" | grep -qE "UPDATE\s+waves\s+SET\s+status\s*=\s*'done'"; then
    block "Never set wave status=done directly -- use validate-wave (requires Thor)"
  fi

  # sqlite3 direct — use cvg CLI
  if echo "$CMD" | grep -qE '(^|[;&[:space:]])sqlite3[[:space:]]'; then
    block "Never use sqlite3 directly -- use cvg CLI commands or daemon API (curl localhost:8420/api/...)"
  fi

  # Pipe to tail/head/grep/cat — warning only
  if echo "$CMD" | grep -qE '\|\s*(tail|head|grep|cat)\b'; then
    warn "Avoid piping to tail/head/grep/cat -- use Read/Grep tools"
  fi

  # CommitLint: conventional commit format check (BLOCK)
  # gate: git commit -m "..."
  if echo "$CMD" | grep -qE '(^|[;&[:space:]])git[[:space:]]+commit.*-m'; then
    local MSG
    MSG=$(echo "$CMD" | sed -n "s/.*-m[[:space:]]*['\"]\\{0,1\\}//p" | sed "s/['\"].*//")
    if [ -n "$MSG" ]; then
      VALID_TYPES="feat|fix|docs|chore|refactor|test|ci|perf|build|style|revert"
      if ! echo "$MSG" | grep -qE "^(${VALID_TYPES})(\([^)]+\))?!?:[[:space:]].+"; then
        block "CommitLint: message must match 'type(scope): message'. Valid types: feat|fix|docs|chore|refactor|test|ci|perf|build|style|revert. Got: $MSG"
      fi
    fi
  fi

  # TestGate: track when cargo test runs — create marker file
  if echo "$CMD" | grep -qE '(^|[;&[:space:]])cargo[[:space:]]+test'; then
    touch "/tmp/.convergio-test-ran-$$" 2>/dev/null || true
    touch "/tmp/.convergio-test-ran" 2>/dev/null || true
  fi

  # TestGate: warn on git commit of .rs files if tests not recently run
  if echo "$CMD" | grep -qE '(^|[;&[:space:]])git[[:space:]]+commit'; then
    if echo "$CMD" | grep -qE '\.(rs)\b' || git diff --cached --name-only 2>/dev/null | grep -qE '\.rs$'; then
      if [ ! -f "/tmp/.convergio-test-ran" ]; then
        warn "TestGate: committing Rust changes but no cargo test run detected in this session. Run cargo test first."
      fi
    fi
  fi

  # git commit — run secret scanner
  if echo "$CMD" | grep -qE '(^|[;&[:space:]])git[[:space:]]+commit'; then
    local ROOT
    ROOT=$(git rev-parse --show-toplevel 2>/dev/null || true)
    if [ -n "$ROOT" ] && [ -x "$ROOT/scripts/platform/secret-scanner.sh" ]; then
      echo "$INPUT" | "$ROOT/scripts/platform/secret-scanner.sh" || true
    fi
  fi
}

guard_edit() {
  local FILE
  FILE=$(file_path)
  [ -z "$FILE" ] && return 0

  # settings.json is protected
  if echo "$FILE" | grep -qE '\.claude/settings\.json$'; then
    block "settings.json is protected"
  fi

  # plan spec files — only task-executor may edit
  if echo "$FILE" | grep -qE '(plan-specs|plans)/.*\.(yaml|yml|md)$'; then
    block "Only task-executor may edit plan spec files"
  fi

  # FailLoud: warn on silent fallback patterns in Rust files
  if echo "$FILE" | grep -qE '\.rs$'; then
    local NS
    NS=$(new_string)
    if echo "$NS" | grep -qE 'unwrap_or_default\(\)'; then
      warn "FailLoud: unwrap_or_default() silently swallows errors. Use expect(), ?, or explicit error handling."
    fi
    if echo "$NS" | grep -qE 'let[[:space:]]+_[[:space:]]*='; then
      warn "FailLoud: 'let _ = ...' discards a Result/value silently. Handle the error explicitly."
    fi
  fi
}

case "$TOOL" in
  bash) guard_bash ;;
  edit) guard_edit ;;
  *)    ;; # unknown tool type — allow
esac
