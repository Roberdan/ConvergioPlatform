#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
# enforce-planner-workflow.sh — PreToolUse: block direct plan-db create/import/done.
set -euo pipefail
INPUT=$(cat); TOOL=$(echo "$INPUT"|jq -r '.tool_name//.toolName//""' 2>/dev/null)
[ "$TOOL" = "EnterPlanMode" ] && echo '{"decision":"block","reason":"Use Skill(planner) not EnterPlanMode."}' && exit 0
[ "$TOOL" != "Bash" ] && [ "$TOOL" != "bash" ] && exit 0
CMD=$(echo "$INPUT"|jq -r '.tool_input.command//.toolArgs.command//""' 2>/dev/null)
[ -z "$CMD" ] && exit 0
echo "$CMD"|grep -qE "plan-db-safe\.sh|planner-create\.sh" && exit 0
echo "$CMD"|grep -qE "^(cd [^;]+[;&] )?git commit|^(echo|printf) " && exit 0
FL=$(echo "$CMD"|head -1)
echo "$FL"|grep -qE "(^|[;&|] *)plan-db\.sh +create " && echo '{"decision":"block","reason":"Use planner-create.sh create."}' && exit 0
echo "$FL"|grep -qE "(^|[;&|] *)plan-db\.sh +import " && echo '{"decision":"block","reason":"Use planner-create.sh import."}' && exit 0
echo "$FL"|grep -qE "(^|[;&|] *)plan-db\.sh +update-task .* done" && echo '{"decision":"block","reason":"Use plan-db-safe.sh to mark done."}' && exit 0
exit 0