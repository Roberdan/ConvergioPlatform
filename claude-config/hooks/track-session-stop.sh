#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# Stop hook — mark session pulse via cvg tracking API
set -euo pipefail
CVG_URL="${CVG_URL:-http://localhost:8420}"
NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
curl -sf -X POST "$CVG_URL/api/tracking/session-state" \
  -H 'Content-Type: application/json' \
  -d "{\"key\":\"last_response_at\",\"value\":\"$NOW\"}" \
  >/dev/null 2>&1 || true
curl -sf -X POST "$CVG_URL/api/tracking/session-state" \
  -H 'Content-Type: application/json' \
  -d "{\"key\":\"session_stop_at\",\"value\":\"$NOW\"}" \
  >/dev/null 2>&1 || true
exit 0
