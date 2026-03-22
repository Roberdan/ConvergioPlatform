#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo. version-check.sh — detect CLI version updates.
set -euo pipefail
DATA_DIR="${CONVERGIO_DATA:-$HOME/.claude/data}"
VERSIONS_JSON="${DATA_DIR}/.cli-versions.json"
CLAUDE_V=$(claude --version 2>/dev/null | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "unknown")
COPILOT_V=$(copilot-cli --version 2>/dev/null | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "unknown")
mkdir -p "$(dirname "$VERSIONS_JSON")"
printf '{"claude":"%s","copilot-cli":"%s"}\n' "$CLAUDE_V" "$COPILOT_V" > "$VERSIONS_JSON"
PREV="${DATA_DIR}/.claude-code-version"
[[ "$CLAUDE_V" == "unknown" ]] && exit 0
[[ ! -f "$PREV" ]] && echo "$CLAUDE_V" > "$PREV" && exit 0
LAST=$(cat "$PREV" 2>/dev/null || echo "unknown")
[[ "$CLAUDE_V" != "$LAST" ]] && echo "$CLAUDE_V" > "$PREV" && echo "{\"notification\":\"Claude Code updated: $LAST -> $CLAUDE_V\"}"
exit 0
