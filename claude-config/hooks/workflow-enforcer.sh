#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
# workflow-enforcer.sh — PreToolUse: state-aware enforcement, zero sqlite3.
set -euo pipefail
INPUT=$(cat); TOOL=$(echo "$INPUT"|jq -r '.tool_name//.toolName//""' 2>/dev/null)
[ "$TOOL" = "EnterPlanMode" ]&&echo '{"decision":"block","reason":"Use Skill(planner) not EnterPlanMode."}' && exit 0
case "$TOOL" in Edit|Write|MultiEdit) PHASE=$(jq -r '.phase//""' "$HOME/.claude/data/workflow-state.json" 2>/dev/null); AG=$(echo "$INPUT"|jq -r '.metadata.agent_type//.agentType//""' 2>/dev/null); [ "$PHASE" = "executing" ]&&[ "$AG" != "task-executor" ]&&[ "$AG" != "thor" ]&&echo '{"decision":"block","reason":"Use Skill(execute) during plan execution."}'; exit 0;; esac
[ "$TOOL" != "Bash" ]&&[ "$TOOL" != "bash" ]&&exit 0
CMD=$(echo "$INPUT"|jq -r '.tool_input.command//.toolArgs.command//""' 2>/dev/null); [ -z "$CMD" ]&&exit 0
echo "$CMD"|grep -qE "plan-db-safe\.sh|planner-create\.sh"&&exit 0; FL=$(echo "$CMD"|head -1)
echo "$FL"|grep -qE "(^|[;&|] *)plan-db\.sh +create "&&echo '{"decision":"block","reason":"Use planner-create.sh create."}' && exit 0
echo "$FL"|grep -qE "(^|[;&|] *)plan-db\.sh +import "&&echo '{"decision":"block","reason":"Use planner-create.sh import."}' && exit 0
echo "$FL"|grep -qE "(^|[;&|] *)plan-db\.sh +update-task .* done"&&echo '{"decision":"block","reason":"Use plan-db-safe.sh to mark done."}' && exit 0
echo "$FL"|grep -qE "wave-worktree\.sh +merge"&&{ N=$(cvg plan tree "$(echo "$CMD"|grep -oE '[0-9]+'|head -1)" 2>/dev/null|jq '[.tasks[]|select(.status=="submitted")]|length' 2>/dev/null||echo 0); [ "${N:-0}" -gt 0 ]&&echo "{\"decision\":\"block\",\"reason\":\"$N tasks submitted — run cvg task validate.\"}"; }||true
exit 0