#!/bin/bash
# subagent-completion-gate.sh — SubagentStop hook
# Catches: uncommitted work in worktrees, auth failures requiring retry
# Why: ~30% of subagents exhaust tool budget before git commit (feedback_context_bloat_prevention.md)
set -euo pipefail

INPUT=$(cat)

# Extract fields from SubagentStop input
AGENT_ID=$(echo "$INPUT" | jq -r '.agent_id // empty' 2>/dev/null)
LAST_MSG=$(echo "$INPUT" | jq -r '.last_assistant_message // empty' 2>/dev/null)

# --- Gate 1: Detect auth failure → block and ask to retry ---
if echo "$LAST_MSG" | grep -qiE "not logged in|please run /login|401|unauthorized|auth.*fail"; then
  echo '{"decision":"block","reason":"Auth token expired. Run /login and retry the task."}'
  exit 0
fi

# --- Gate 2: Check for uncommitted work in agent worktree ---
ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "")
if [[ -z "$ROOT" ]]; then
  exit 0
fi

# Find worktree matching this agent ID (pattern: agent-<first8chars>)
SHORT_ID="${AGENT_ID:0:8}"
WORKTREE=""
for wt in "$ROOT/.claude/worktrees"/agent-"$SHORT_ID"*; do
  if [[ -d "$wt" ]]; then
    WORKTREE="$wt"
    break
  fi
done

if [[ -z "$WORKTREE" ]]; then
  exit 0
fi

# Check for uncommitted changes
DIRTY=$(cd "$WORKTREE" && git status --porcelain 2>/dev/null | head -1)
if [[ -n "$DIRTY" ]]; then
  # Auto-commit the agent's work instead of losing it
  cd "$WORKTREE"
  BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
  git add -A 2>/dev/null
  git commit -m "fix: auto-commit uncommitted subagent work ($BRANCH)

Agent $AGENT_ID exhausted tool budget before committing.
Auto-committed by subagent-completion-gate.sh to prevent work loss.

Co-Authored-By: subagent-completion-gate <noreply@convergio.dev>" 2>/dev/null || true

  echo "AUTO-COMMITTED: Agent $AGENT_ID had uncommitted changes in $WORKTREE" >&2
fi

exit 0
