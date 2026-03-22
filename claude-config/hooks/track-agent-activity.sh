#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# PostToolUse hook — track agent/tool activity via cvg tracking API
set -euo pipefail
CVG_URL="${CVG_URL:-http://localhost:8420}"
INPUT=$(cat 2>/dev/null || true)
[[ -z "$INPUT" ]] && exit 0
TOOL=$(echo "$INPUT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('tool_name','unknown'))" 2>/dev/null || echo "unknown")
TS=$(date +%s)
AGENT_ID="tool-${TOOL}-${TS}"
curl -sf -X POST "$CVG_URL/api/tracking/agent-activity" \
  -H 'Content-Type: application/json' \
  -d "{\"agent_id\":\"$AGENT_ID\",\"action\":\"$TOOL\",\"status\":\"completed\",\"host\":\"$(hostname -s 2>/dev/null || echo local)\"}" \
  >/dev/null 2>&1 || true
exit 0
