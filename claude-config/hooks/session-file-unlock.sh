#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# SessionStop: release all file locks held by this session on exit.
# Version: 2.1.0
set -uo pipefail
INPUT=$(cat)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null)
[[ -z "$SESSION_ID" ]] && exit 0
{ command -v file-lock.sh >/dev/null 2>&1 || exit 0
  RESULT=$(file-lock.sh release-session "$SESSION_ID" 2>/dev/null || true)
  COUNT=$(echo "$RESULT" | jq -r '.released_count // 0' 2>/dev/null || true)
  [[ "${COUNT:-0}" -gt 0 ]] && echo "[session-file-unlock] Released $COUNT lock(s)" \
    >>"${MYCONVERGIO_HOME:-$HOME/.myconvergio}/logs/hooks.log" 2>/dev/null || true
} &
exit 0
