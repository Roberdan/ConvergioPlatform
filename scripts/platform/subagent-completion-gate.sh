#!/bin/bash
# subagent-completion-gate.sh — SubagentStop hook
# Catches: uncommitted work in worktrees, auth failures, missing evidence
# Why: ~30% of subagents exhaust tool budget before git commit (feedback_context_bloat_prevention.md)
set -euo pipefail

INPUT=$(cat)

# Extract fields from SubagentStop input
AGENT_ID=$(echo "$INPUT" | jq -r '.agent_id // empty' 2>/dev/null)
LAST_MSG=$(echo "$INPUT" | jq -r '.last_assistant_message // empty' 2>/dev/null)

# --- EvidenceGate: warn when "done/completed/submitted" claimed without evidence ---
# WARNING (not block) — avoids breaking existing flows, surfaces missing evidence.
# Why: v20 audit found 8/17 features were fake (feedback_fake_implementations.md).
CLAIMS_DONE=false
EVIDENCE_FOUND=false

if echo "$LAST_MSG" | grep -qiE '\b(done|completed|submitted|finished|complete)\b'; then
  CLAIMS_DONE=true
fi

if echo "$LAST_MSG" | grep -qiE 'cargo test|test result|tests passed|passing|✓|curl.*output|exit 0|ok \([0-9]+ test|npx vitest|pytest|assertion'; then
  EVIDENCE_FOUND=true
fi

if $CLAIMS_DONE && ! $EVIDENCE_FOUND; then
  echo "WARNING: EvidenceGate — agent claims done/completed but no evidence found (cargo test, test results, curl output, etc.). Verify before marking submitted." >&2
fi

# --- Gate 1: Detect auth failure → block and ask to retry ---
if echo "$LAST_MSG" | grep -qiE "not logged in|please run /login|401|unauthorized|auth.*fail"; then
  echo '{"decision":"block","reason":"Auth token expired. Run /login and retry the task."}'
  exit 0
fi

# --- Gate 2: AgentIdentity warning — CONVERGIO_AGENT_NAME should be set ---
if [ -z "${CONVERGIO_AGENT_NAME:-}" ]; then
  echo "WARNING: AgentIdentity — CONVERGIO_AGENT_NAME is not set. Register with: cvg agent start <name>. Session is invisible in cvg who agents." >&2
fi

# --- Gate 3: Check for uncommitted work in agent worktree ---
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
