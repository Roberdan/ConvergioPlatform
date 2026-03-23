#!/usr/bin/env bash
# Workspace event hook — registers file operations in daemon event log.
# Installed as PostToolUse hook. Daemon-down safe (--connect-timeout 1, || true).
set -euo pipefail

DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"
AGENT="${CLAUDE_AGENT:-unknown}"
TOOL_NAME="${HOOK_TOOL_NAME:-}"
FILE_PATH="${HOOK_FILE_PATH:-}"

# Only track file write operations
case "$TOOL_NAME" in
  Write|Edit) ACTION="file_write" ;;
  *) exit 0 ;;  # Skip non-file tools
esac

# Skip if no file path
[[ -z "$FILE_PATH" ]] && exit 0

# Detect workspace from cwd — query daemon for active workspaces
CWD="$(pwd)"
WORKSPACE_ID=$(curl -sf --connect-timeout 1 "${DAEMON_URL}/api/workspace/list" 2>/dev/null \
  | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    cwd = '$CWD'
    for ws in data.get('workspaces', []):
        if cwd.startswith(ws.get('path', '')):
            print(ws['workspace_id'])
            break
except:
    pass
" 2>/dev/null || true)

# Skip if not in a known workspace
[[ -z "$WORKSPACE_ID" ]] && exit 0

# Record event (fire and forget, daemon-down safe)
curl -sf --connect-timeout 1 -X POST "${DAEMON_URL}/api/workspace/events/record" \
  -H "Content-Type: application/json" \
  -d "{\"workspace_id\":\"$WORKSPACE_ID\",\"agent\":\"$AGENT\",\"action\":\"$ACTION\",\"file_path\":\"$FILE_PATH\"}" \
  >/dev/null 2>&1 || true
