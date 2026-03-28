#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# PreCompact hook — checkpoint active plan + spawn copilot continuation
set -euo pipefail
CVG_URL="${CVG_URL:-http://localhost:8420}"
SCRIPTS="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"

# 1. Track compaction event
curl -sf -X POST "$CVG_URL/api/tracking/compaction" \
  -H 'Content-Type: application/json' \
  -d "{\"event_type\":\"compaction\",\"context\":\"PreCompact\"}" \
  >/dev/null 2>&1 || true

# 2. Find active plan and checkpoint it
ACTIVE_PLAN="$(curl -sf "$CVG_URL/api/plan-db/list" 2>/dev/null \
  | python3 -c "
import json,sys
plans=json.load(sys.stdin).get('plans',[])
doing=[p for p in plans if p['status']=='doing']
print(doing[0]['id'] if doing else '')
" 2>/dev/null || echo '')"

if [ -n "$ACTIVE_PLAN" ]; then
  # Checkpoint plan state
  cvg checkpoint save "$ACTIVE_PLAN" 2>/dev/null || true

  # Spawn copilot continuation as new tab in Convergio tmux session
  if tmux has-session -t Convergio 2>/dev/null; then
    tmux new-window -t Convergio -n "plan-${ACTIVE_PLAN}" \
      "sleep 10 && ${SCRIPTS}/copilot-plan-runner.sh ${ACTIVE_PLAN}" \
      2>/dev/null || true
  else
    tmux new-session -d -s Convergio -n "plan-${ACTIVE_PLAN}" \
      "sleep 10 && ${SCRIPTS}/copilot-plan-runner.sh ${ACTIVE_PLAN}" \
      2>/dev/null || true
  fi
fi
exit 0
