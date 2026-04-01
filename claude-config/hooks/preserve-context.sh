#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# PreCompact: checkpoint active plan + inject recovery context. Version: 2.0.0
set -euo pipefail
INPUT=$(cat)
TRANSCRIPT=$(echo "$INPUT" | jq -r '.transcript_path // empty' 2>/dev/null)
[[ -z "$TRANSCRIPT" || ! -f "$TRANSCRIPT" ]] && exit 0
RESULT=$(command -v cvg >/dev/null 2>&1 && cvg checkpoint save-auto 2>/dev/null || true)
[[ -n "$RESULT" && -f "$RESULT" ]] || exit 0
PRESERVED=$(cat "$RESULT")
[[ -z "$PRESERVED" ]] && exit 0
PRESERVED="${PRESERVED}
## Recovery: cvg checkpoint restore <id> -> cvg plan show <id> -> resume"
jq -n --arg ctx "$PRESERVED" '{"additionalContext":$ctx}'
