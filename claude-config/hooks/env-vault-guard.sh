#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
# env-vault-guard.sh — PreToolUse: blocks git commit when staged files contain secrets.
set -euo pipefail
INPUT=$(cat); TOOL=$(echo "$INPUT"|jq -r '.toolName//.tool_name//""' 2>/dev/null)
[[ "$TOOL" != "bash" && "$TOOL" != "shell" && "$TOOL" != "Bash" ]]&&exit 0
CMD=$(echo "$INPUT"|jq -r '.toolArgs.command//.tool_input.command//""' 2>/dev/null)
echo "$CMD"|grep -qE '(^|[;&[:space:]])git[[:space:]]+commit([[:space:]]|$)'||exit 0
PATTERNS='API_KEY=|SECRET=|PASSWORD=|CONNECTION_STRING=|private_key|token'
FILES=$(git diff --cached --name-only --diff-filter=ACM 2>/dev/null||true)
for f in $FILES; do
  [ -f "$f" ]||continue
  grep -qE "$PATTERNS" "$f" 2>/dev/null&&jq -n --arg r "BLOCKED: secret-like pattern in staged file: $f" '{permissionDecision:"deny",permissionDecisionReason:$r}'&&exit 0
done
[[ -f .gitignore ]]&&grep -q '^\.env$' .gitignore 2>/dev/null||echo "[WARNING] .env not in .gitignore" >&2
exit 0