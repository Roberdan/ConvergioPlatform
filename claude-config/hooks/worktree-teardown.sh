#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# WorktreeRemove: release file locks held by this worktree session.
# Version: 2.0.0
set -euo pipefail
WORKTREE_PATH="${1:-}"
[[ -z "$WORKTREE_PATH" ]] && exit 0
SESSION_ID=$(echo "$WORKTREE_PATH" | tr '/' '_' | tr -cd '[:alnum:]_')
if command -v file-lock.sh >/dev/null 2>&1; then
  file-lock.sh release-session "$SESSION_ID" 2>/dev/null || true
fi
