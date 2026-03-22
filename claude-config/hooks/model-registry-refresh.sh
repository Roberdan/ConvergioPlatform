#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# SessionStart: snapshot CLI versions to disk for daemon model registry.
# Version: 2.0.0
set -euo pipefail
MYCONVERGIO_HOME="${MYCONVERGIO_HOME:-$HOME/.myconvergio}"
VERSIONS_JSON="${MYCONVERGIO_HOME}/data/.cli-versions.json"
mkdir -p "$(dirname "$VERSIONS_JSON")"
CLAUDE_V=$(claude --version 2>/dev/null | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "unknown")
printf '{"claude":"%s","checked":"%s"}\n' "$CLAUDE_V" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$VERSIONS_JSON"
