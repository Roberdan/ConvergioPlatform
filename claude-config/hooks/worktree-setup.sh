#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# WorktreeCreate: symlink .env* files and run npm install in new worktree.
# Version: 2.0.0
set -euo pipefail
WORKTREE_PATH="${1:-}"
[[ -z "$WORKTREE_PATH" ]] && exit 0
MAIN_REPO=$(git -C "$WORKTREE_PATH" rev-parse --path-format=absolute --git-common-dir 2>/dev/null | sed 's|/.git$||' || true)
[[ -z "$MAIN_REPO" || "$MAIN_REPO" == "$WORKTREE_PATH" ]] && exit 0
for ENV_FILE in "$MAIN_REPO"/.env*; do
  [[ -e "$ENV_FILE" ]] || continue
  TARGET="$WORKTREE_PATH/$(basename "$ENV_FILE")"
  [[ ! -e "$TARGET" && ! -L "$TARGET" ]] && ln -s "$ENV_FILE" "$TARGET" 2>/dev/null || true
done
[[ -f "$WORKTREE_PATH/package.json" ]] && (cd "$WORKTREE_PATH" && npm install --silent 2>/dev/null) || true
