#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# Shared hooks library — DRY utilities for all hooks. Source: source "$HOOK_DIR/lib/common.sh"
# Version: 2.0.0
set -euo pipefail

DAEMON_URL="${CLAUDE_DAEMON_URL:-http://localhost:8420}"
MYCONVERGIO_HOME="${MYCONVERGIO_HOME:-$HOME/.myconvergio}"
HOOKS_LOG="${MYCONVERGIO_HOOKS_LOG:-${MYCONVERGIO_HOME}/logs/hooks.log}"

have_bin() { command -v "$1" >/dev/null 2>&1; }
log_hook() { mkdir -p "$(dirname "$HOOKS_LOG")" 2>/dev/null; echo "[$(date '+%H:%M:%S')] [$1] $2" >>"$HOOKS_LOG" 2>/dev/null & }
json_field() { jq -r "${1} // empty" 2>/dev/null; }

# POST to daemon tracking API (non-blocking, uses HTTP not DB)
track_api() {
  local endpoint="$1" payload="$2"
  have_bin curl || return 0
  curl -sf -X POST -H 'Content-Type: application/json' \
    -d "$payload" "${DAEMON_URL}${endpoint}" >/dev/null 2>&1 &
}
