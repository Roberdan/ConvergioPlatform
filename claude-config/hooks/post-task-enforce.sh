#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# PostToolUse: after plan-db-safe.sh done, auto-checkpoint + remind Thor.
# Version: 2.0.0
set -euo pipefail
INPUT=$(cat)
TOOL=$(echo "$INPUT" | jq -r '.tool_name // .toolName // ""' 2>/dev/null)
[[ "$TOOL" != "Bash" && "$TOOL" != "bash" ]] && exit 0
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // .toolArgs.command // ""' 2>/dev/null)
echo "$CMD" | grep -qE 'plan-db-safe\.sh.*update-task.*done' || exit 0
TASK_ID=$(echo "$CMD" | grep -oE '[0-9]+' | head -1)
PLAN_ID=$(echo "$CMD" | grep -oE '[0-9]+' | sed -n '2p')
[[ -n "$PLAN_ID" ]] && command -v plan-checkpoint.sh >/dev/null 2>&1 && plan-checkpoint.sh save "$PLAN_ID" 2>/dev/null || true
jq -n --arg t "$TASK_ID" '{"decision":"allow","notification":("NEXT: checkpoint → plan-db.sh validate-task "+$t+" {plan_id} → next task")}'
