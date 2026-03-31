#!/usr/bin/env bash
# main-guard.sh — BLOCK writes to daemon/src/ when CWD is on the main branch.
# WHY: 2026-03-31 incident — copilot dirtied 467 files on main directly.
# All code changes must happen in worktrees, never on main.
# Triggered by: PreToolUse Edit/Write hooks.
set -euo pipefail

INPUT=$(cat)
FILE=$(echo "$INPUT" | jq -r '.file_path // .input.file_path // empty' 2>/dev/null)
[ -z "$FILE" ] && exit 0

# Only guard daemon source files (the most critical path)
echo "$FILE" | grep -qE "daemon/src/|daemon/tests/" || exit 0

# Check if we're in the main repo on the main branch
TOPLEVEL=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
BRANCH=$(git -C "$TOPLEVEL" rev-parse --abbrev-ref HEAD 2>/dev/null) || exit 0

# Allow if we're in a worktree (not the main checkout)
COMMON=$(git -C "$TOPLEVEL" rev-parse --git-common-dir 2>/dev/null) || exit 0
GITDIR=$(git -C "$TOPLEVEL" rev-parse --git-dir 2>/dev/null) || exit 0
[ "$COMMON" != "$GITDIR" ] && exit 0  # worktree — allow

# Block if on main/master
if [ "$BRANCH" = "main" ] || [ "$BRANCH" = "master" ]; then
  echo "BLOCKED: MainGuard — cannot modify $FILE on branch '$BRANCH'." >&2
  echo "Create a worktree: git worktree add --detach /path HEAD" >&2
  exit 2
fi

exit 0
