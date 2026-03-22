#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# PreCompact hook — log compaction event via cvg tracking API
set -euo pipefail
CVG_URL="${CVG_URL:-http://localhost:8420}"
SESSION_ID="${CLAUDE_SESSION_ID:-unknown-$(date +%s)}"
curl -sf -X POST "$CVG_URL/api/tracking/compaction" \
  -H 'Content-Type: application/json' \
  -d "{\"session_id\":\"$SESSION_ID\",\"event_type\":\"compaction\",\"context\":\"PreCompact hook\"}" \
  >/dev/null 2>&1 || true
curl -sf -X POST "$CVG_URL/api/tracking/session-state" \
  -H 'Content-Type: application/json' \
  -d "{\"key\":\"last_compaction_at\",\"value\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" \
  >/dev/null 2>&1 || true
exit 0
