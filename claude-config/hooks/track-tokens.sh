#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
set -euo pipefail
CVG_URL="${CVG_URL:-http://localhost:8420}"
INPUT=$(cat 2>/dev/null || true)
[[ -z "$INPUT" ]] && exit 0
AGENT=$(echo "$INPUT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('agent','claude-code'))" 2>/dev/null || echo "claude-code")
MODEL=$(echo "$INPUT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('model','unknown'))" 2>/dev/null || echo "unknown")
IN=$(echo "$INPUT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('input_tokens',0))" 2>/dev/null || echo "0")
OUT=$(echo "$INPUT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('output_tokens',0))" 2>/dev/null || echo "0")
curl -sf -X POST "$CVG_URL/api/tracking/tokens" \
  -H 'Content-Type: application/json' \
  -d "{\"agent\":\"$AGENT\",\"model\":\"$MODEL\",\"input_tokens\":$IN,\"output_tokens\":$OUT,\"cost_usd\":0}" \
  >/dev/null 2>&1 || true
exit 0
