#!/usr/bin/env bash
# block-branch-creation.sh — Claude Code hook: blocks git branch creation.
# WHY: All worktrees must use detached HEAD. Branches are managed exclusively
#      by the workspace release pipeline to prevent accidental branch pollution.
# Triggered by: Claude Code Bash tool hook (PreToolUse / bash commands).

set -euo pipefail

# Read the command from stdin (Claude Code hook protocol passes JSON).
# For direct invocation the command is the first argument.
if [[ $# -ge 1 ]]; then
  CMD="$*"
else
  # Parse JSON input: {"tool":"Bash","input":{"command":"..."}}
  INPUT=$(cat)
  CMD=$(echo "$INPUT" | grep -o '"command":"[^"]*"' | head -1 | sed 's/"command":"//;s/"$//')
fi

# Allow list — checked before deny list.
# git branch with no sub-command args (listing), or delete flags.
if echo "$CMD" | grep -qE 'git branch[[:space:]]*(-[dD]|-v|--list|-a|-r|$)'; then
  exit 0
fi

# git worktree add --detach is the only allowed worktree creation form.
if echo "$CMD" | grep -q 'git worktree add'; then
  if echo "$CMD" | grep -q -- '--detach'; then
    exit 0
  fi
  # worktree add without --detach creates a branch — block it.
  echo "HOOK BLOCKED: 'git worktree add' without --detach creates a branch." >&2
  echo "Use: git worktree add --detach <path> HEAD" >&2
  exit 1
fi

# Deny list — branch creation patterns.
if echo "$CMD" | grep -qE 'git (branch [^-]|checkout -b|switch -c|switch --create)'; then
  echo "HOOK BLOCKED: Branch creation is forbidden. Use 'git worktree add --detach <path> HEAD' instead." >&2
  echo "Branches are created only by the workspace release pipeline at merge time." >&2
  exit 1
fi

exit 0
