#!/usr/bin/env bash
# Agent session registration hook for Claude Code / Copilot CLI.
# Called by SessionStart (register) and Stop (deregister) hooks.
# Usage: agent-session-hook.sh {start|complete} [type]
# Type: claude (default), copilot, executor
set -euo pipefail

ACTION="${1:-start}"
AGENT_TYPE="${2:-claude}"
IDENTITY="${AGENT_TYPE}-$(hostname -s)-$$"
CVG_BIN="${CVG_BIN:-cvg}"
DAEMON_URL="${CONVERGIO_API_URL:-http://localhost:8420}"

# Only register if daemon is reachable (non-blocking, 2s timeout)
if ! curl -sf --max-time 2 "${DAEMON_URL}/api/health" >/dev/null 2>&1; then
    exit 0
fi

case "$ACTION" in
    start)
        "$CVG_BIN" agent start "$IDENTITY" 2>/dev/null || true
        ;;
    complete)
        "$CVG_BIN" agent complete "$IDENTITY" 2>/dev/null || true
        ;;
    *)
        echo "Usage: agent-session-hook.sh {start|complete} [claude|copilot|executor]" >&2
        exit 1
        ;;
esac
