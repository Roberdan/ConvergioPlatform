#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
set -euo pipefail
CVG_URL="${CVG_URL:-http://localhost:8420}"
INPUT=$(cat 2>/dev/null || true)
[[ -z "$INPUT" ]] && exit 0
TRANSCRIPT=$(echo "$INPUT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('transcript_path',''))" 2>/dev/null || true)
[[ -z "$TRANSCRIPT" || ! -f "$TRANSCRIPT" ]] && exit 0
STATS=$(jq -s '{input:(map(.message.usage.input_tokens//0)|add),output:(map(.message.usage.output_tokens//0)|add),model:(.[0].message.model//"unknown")}' "$TRANSCRIPT" 2>/dev/null || true)
[[ -z "$STATS" ]] && exit 0
IN=$(echo "$STATS" | jq -r '.input'); OUT=$(echo "$STATS" | jq -r '.output'); MODEL=$(echo "$STATS" | jq -r '.model')
[[ $((IN+OUT)) -eq 0 ]] && exit 0
curl -sf -X POST "$CVG_URL/api/tracking/tokens" -H 'Content-Type: application/json' \
  -d "{\"agent\":\"claude-code\",\"model\":\"$MODEL\",\"input_tokens\":$IN,\"output_tokens\":$OUT,\"cost_usd\":0}" >/dev/null 2>&1 || true
exit 0
