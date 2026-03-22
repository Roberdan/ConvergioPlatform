#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# SessionStop: kill orphaned processes when a session ends.
# Version: 2.0.0
set -uo pipefail
INPUT=$(cat)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null)
[[ -z "$SESSION_ID" ]] && exit 0
{ command -v session-reaper.sh >/dev/null 2>&1 && session-reaper.sh --max-age 0 2>/dev/null; } &
exit 0
