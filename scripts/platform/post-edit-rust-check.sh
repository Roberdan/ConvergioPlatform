#!/usr/bin/env bash
set -euo pipefail

# PostToolUse hook for Edit on .rs files: runs cargo check
# Reads hook input JSON from stdin

INPUT=$(cat)
FILE=$(echo "$INPUT" | jq -r '.file_path // empty' 2>/dev/null)

# Only run for Rust files
if [ -z "$FILE" ] || ! echo "$FILE" | grep -qE '\.rs$'; then
  exit 0
fi

ROOT=$(git rev-parse --show-toplevel 2>/dev/null || true)
if [ -z "$ROOT" ] || [ ! -f "$ROOT/daemon/Cargo.toml" ]; then
  exit 0
fi

# Worktree isolation: use a separate target dir so cargo check does not
# lock the main repo's target/ and crash the running daemon binary.
if [ "$(git rev-parse --is-inside-work-tree 2>/dev/null)" = "true" ]; then
  _wt_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
  _main_wt="$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null | sed 's|/\.git$||' || true)"
  if [ -n "$_main_wt" ] && [ "$_wt_root" != "$_main_wt" ]; then
    export CARGO_TARGET_DIR="/tmp/convergio-target-$(basename "$_wt_root")"
  fi
fi

OUTPUT=$(cd "$ROOT/daemon" && cargo check --quiet 2>&1 | head -10)
if [ -n "$OUTPUT" ]; then
  echo "cargo check errors after edit:" >&2
  echo "$OUTPUT" >&2
fi

exit 0
