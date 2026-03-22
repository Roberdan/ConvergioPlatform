#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# SubagentStart: inject constitution + security context into all subagents.
# Version: 2.0.0
set -euo pipefail
INPUT=$(cat)
AGENT=$(echo "$INPUT" | jq -r '.agent_type // empty' 2>/dev/null)
[[ -z "$AGENT" ]] && exit 0
CTX="## Constitution\n- Verify before claim. Act, don't suggest. Max 250 lines/file.\n- English only. MPL-2.0."
case "$AGENT" in task-executor*|Bash|app-release-manager*)
  CTX="${CTX}\n## Security\n- Parameterized queries. No secrets in code. CSP+TLS 1.2+. RBAC." ;;
esac
CTX="${CTX}\n## Platform\n- Daemon :8420. LSP navigation. isolation:worktree. CRDT sync."
jq -n --arg ctx "$CTX" '{"additionalContext":$ctx}'
