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

  curl -sf -X POST "$CVG_URL/api/delegate/spawn" \
    -H 'Content-Type: application/json' \
    -d "$(jq -n \
      --arg session "Convergio" \
      --arg window "plan-${ACTIVE_PLAN}" \
      --arg cwd "$PWD" \
      --arg command "sleep 10 && ${SCRIPTS}/copilot-plan-runner.sh ${ACTIVE_PLAN}" \
      '{peer:"local",tmux_session:$session,tmux_window:$window,cwd:$cwd,command:$command}')" \
    >/dev/null 2>&1 || true
fi
exit 0
