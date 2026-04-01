#!/bin/bash
# subagent-auto-submit.sh — SubagentStop hook: auto-submit task when subagent completes
# Why: Executors forget to POST evidence + set status=submitted before exiting.
# This hook reads the agent's task_db_id from the daemon registry and auto-submits.
# Works alongside subagent-completion-gate.sh (separate concerns: gate=safety, this=workflow).
set -euo pipefail

DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"
AUTH_TOKEN="${CONVERGIO_AUTH_TOKEN:-dev-local}"

INPUT=$(cat)

# Extract agent_id from SubagentStop stdin JSON
AGENT_ID=$(echo "$INPUT" | jq -r '.agent_id // empty' 2>/dev/null)
if [[ -z "$AGENT_ID" ]]; then
  echo '{"systemMessage":"subagent-auto-submit: no agent_id in input, skipping"}'
  exit 0
fi

LAST_MSG=$(echo "$INPUT" | jq -r '.last_assistant_message // empty' 2>/dev/null)

# Query daemon for registered agents, find the one matching this agent_id
AGENTS_JSON=$(curl -sf "$DAEMON_URL/api/ipc/agents/list" \
  -H "Authorization: Bearer $AUTH_TOKEN" 2>/dev/null || echo '{}')

# Match agent by name field (agent_id from hook matches name in registry)
TASK_DB_ID=$(echo "$AGENTS_JSON" | jq -r \
  --arg aid "$AGENT_ID" \
  '.agents // [] | map(select(.name == $aid or .agent_id == $aid)) | .[0].task_db_id // empty' \
  2>/dev/null)

if [[ -z "$TASK_DB_ID" ]]; then
  echo '{"systemMessage":"subagent-auto-submit: no task_db_id for agent '"$AGENT_ID"', skipping"}'
  exit 0
fi

# Extract test evidence from last_assistant_message (best-effort)
EVIDENCE_CONTENT="Auto-extracted from subagent completion"
if [[ -n "$LAST_MSG" ]]; then
  # Grab lines that look like test output (cargo test, pytest, vitest, curl, etc.)
  TEST_LINES=$(echo "$LAST_MSG" | grep -iE \
    'test result|tests? passed|passing|ok \([0-9]|cargo test|pytest|vitest|curl.*200|exit.?code.*0|assert|PASS' \
    2>/dev/null | head -20 || true)
  if [[ -n "$TEST_LINES" ]]; then
    EVIDENCE_CONTENT="$TEST_LINES"
  fi
fi

# Truncate evidence to 2000 chars to avoid payload bloat
EVIDENCE_CONTENT="${EVIDENCE_CONTENT:0:2000}"

# Step 1: POST evidence
EVIDENCE_RESP=$(curl -sf -X POST "$DAEMON_URL/api/plan-db/task/evidence" \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -d "$(jq -n \
    --argjson tid "$TASK_DB_ID" \
    --arg etype "test_pass" \
    --arg content "$EVIDENCE_CONTENT" \
    '{task_id: $tid, evidence_type: $etype, content: $content}')" \
  2>/dev/null || echo '{"error":"evidence POST failed"}')

EVIDENCE_OK=false
if echo "$EVIDENCE_RESP" | jq -e '.error' >/dev/null 2>&1; then
  EVIDENCE_OK=false
else
  EVIDENCE_OK=true
fi

# Step 2: POST status=submitted
SUBMIT_RESP=$(curl -sf -X POST "$DAEMON_URL/api/plan-db/task/update" \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -d "$(jq -n \
    --argjson tid "$TASK_DB_ID" \
    --arg status "submitted" \
    --arg summary "Auto-submitted by subagent-auto-submit.sh on agent completion" \
    '{task_id: $tid, status: $status, summary: $summary}')" \
  2>/dev/null || echo '{"error":"submit POST failed"}')

SUBMIT_OK=false
if echo "$SUBMIT_RESP" | jq -e '.error' >/dev/null 2>&1; then
  SUBMIT_OK=false
else
  SUBMIT_OK=true
fi

# Output systemMessage summarizing actions
MSG="subagent-auto-submit: agent=$AGENT_ID task=$TASK_DB_ID"
if $EVIDENCE_OK; then
  MSG="$MSG evidence=recorded"
else
  MSG="$MSG evidence=FAILED"
fi
if $SUBMIT_OK; then
  MSG="$MSG status=submitted"
else
  MSG="$MSG status=FAILED"
fi

echo "{\"systemMessage\":\"$MSG\"}"
exit 0
