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

OUTPUT=$(cd "$ROOT/daemon" && cargo check --quiet 2>&1 | head -10)
if [ -n "$OUTPUT" ]; then
  echo "cargo check errors after edit:" >&2
  echo "$OUTPUT" >&2
fi

exit 0
