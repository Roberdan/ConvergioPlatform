#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
# worktree-guard.sh — BLOCKS execution if not in the correct worktree.
# Usage: worktree-guard.sh <expected_worktree_path>
set -euo pipefail
EXPECTED="${1:-}"; [ -z "$EXPECTED" ]&&echo "WORKTREE_VIOLATION: no expected path" >&2 && exit 1
EXPECTED="${EXPECTED/#\~/$HOME}"
GIT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null)||{ echo "WORKTREE_VIOLATION: not a git repo" >&2; exit 1; }
BRANCH=$(git branch --show-current 2>/dev/null||echo DETACHED)
[[ "$BRANCH" == "main" || "$BRANCH" == "master" ]]&&{ echo "WORKTREE_VIOLATION: on protected branch '$BRANCH'" >&2; echo "  Expected: $EXPECTED" >&2; echo "  Actual: $GIT_ROOT (branch: $BRANCH)" >&2; echo "  ACTION: cd to your assigned worktree" >&2; exit 1; }
EXPECTED_REAL=$(cd "$EXPECTED" 2>/dev/null&&pwd -P||echo "$EXPECTED"); CURRENT_REAL=$(pwd -P); GIT_ROOT_REAL=$(cd "$GIT_ROOT"&&pwd -P)
[[ "$GIT_ROOT_REAL" != "$EXPECTED_REAL" && "$CURRENT_REAL" != "$EXPECTED_REAL" ]]&&{ echo "WORKTREE_VIOLATION: wrong worktree (expected: $EXPECTED, actual: $GIT_ROOT_REAL)" >&2; exit 1; }
git worktree list --porcelain 2>/dev/null|grep -qF "worktree $GIT_ROOT_REAL"||{ echo "WORKTREE_VIOLATION: not a registered git worktree: $GIT_ROOT_REAL" >&2; exit 1; }
echo "WORKTREE_OK: $BRANCH @ $GIT_ROOT_REAL"