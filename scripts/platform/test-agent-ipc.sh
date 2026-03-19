#!/usr/bin/env bash
# test-agent-ipc.sh — E2E test for agent IPC communication
# Output: TAP format (ok N description / not ok N description)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONVERGIO_DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"
CURL_OPTS=(-s --max-time 3)

TEST_NUM=0
FAILURES=0
DAEMON_UP=false

tap_ok()   { TEST_NUM=$((TEST_NUM + 1)); echo "ok $TEST_NUM - $1"; }
tap_fail() { TEST_NUM=$((TEST_NUM + 1)); FAILURES=$((FAILURES + 1)); echo "not ok $TEST_NUM - $1"; }
tap_skip() { TEST_NUM=$((TEST_NUM + 1)); echo "ok $TEST_NUM - SKIP $1"; }

# shellcheck disable=SC2329
cleanup() {
  if [[ "$DAEMON_UP" == "true" ]]; then
    # Best-effort unregister test agents
    local body
    for agent in e2e-claude e2e-copilot e2e-offline; do
      body=$(printf '{"agent_id":"%s","host":"%s"}' "$agent" "$(hostname)")
      curl "${CURL_OPTS[@]}" -X POST \
        "${CONVERGIO_DAEMON_URL}/api/ipc/agents/unregister" \
        -H 'Content-Type: application/json' \
        -d "$body" >/dev/null 2>&1 || true
    done
  fi
}
trap cleanup EXIT

# --- Test 1: Daemon reachable ---
test_daemon_reachable() {
  local resp
  resp=$(curl "${CURL_OPTS[@]}" "${CONVERGIO_DAEMON_URL}/api/ipc/status" 2>/dev/null) || resp=""
  if [[ -n "$resp" ]] && echo "$resp" | grep -qE '"ok"|agents_active'; then
    tap_ok "daemon reachable at ${CONVERGIO_DAEMON_URL}"
    DAEMON_UP=true
  else
    tap_fail "daemon not reachable at ${CONVERGIO_DAEMON_URL}"
  fi
}

skip_remaining() {
  tap_skip "agent registration (claude) - daemon not running"
  tap_skip "agent registration (copilot) - daemon not running"
  tap_skip "agent messaging - daemon not running"
  tap_skip "broadcast messaging - daemon not running"
  tap_skip "heartbeat - daemon not running"
  tap_skip "agent unregistration - daemon not running"
}

# --- Test 2: Agent registration (Claude) ---
test_register_claude() {
  bash "$SCRIPT_DIR/agent-bridge.sh" --register --name e2e-claude --type claude 2>/dev/null
  local resp
  resp=$(curl "${CURL_OPTS[@]}" "${CONVERGIO_DAEMON_URL}/api/ipc/agents" 2>/dev/null) || resp=""
  if echo "$resp" | grep -q "e2e-claude"; then
    tap_ok "agent registration (claude)"
  else
    tap_fail "agent registration (claude) - not found in /api/ipc/agents"
  fi
}

# --- Test 3: Agent registration (Copilot) ---
test_register_copilot() {
  bash "$SCRIPT_DIR/copilot-bridge.sh" --register --name e2e-copilot 2>/dev/null
  local resp
  resp=$(curl "${CURL_OPTS[@]}" "${CONVERGIO_DAEMON_URL}/api/ipc/agents" 2>/dev/null) || resp=""
  if echo "$resp" | grep -q "e2e-copilot"; then
    tap_ok "agent registration (copilot)"
  else
    tap_fail "agent registration (copilot) - not found in /api/ipc/agents"
  fi
}

# --- Test 4: Agent messaging ---
test_messaging() {
  local send_body resp
  send_body='{"sender_name":"e2e-claude","channel":"test","content":"hello from claude"}'
  curl "${CURL_OPTS[@]}" -X POST "${CONVERGIO_DAEMON_URL}/api/ipc/send" \
    -H 'Content-Type: application/json' -d "$send_body" >/dev/null 2>&1
  resp=$(curl "${CURL_OPTS[@]}" "${CONVERGIO_DAEMON_URL}/api/ipc/messages?channel=test" 2>/dev/null) || resp=""
  if echo "$resp" | grep -q "hello from claude"; then
    tap_ok "agent messaging (send + receive)"
  else
    tap_fail "agent messaging - 'hello from claude' not in /api/ipc/messages?channel=test"
  fi
}

# --- Test 5: Broadcast messaging ---
test_broadcast() {
  local send_body resp
  send_body='{"sender_name":"e2e-claude","channel":"planning","content":"broadcast test"}'
  curl "${CURL_OPTS[@]}" -X POST "${CONVERGIO_DAEMON_URL}/api/ipc/send" \
    -H 'Content-Type: application/json' -d "$send_body" >/dev/null 2>&1
  resp=$(curl "${CURL_OPTS[@]}" "${CONVERGIO_DAEMON_URL}/api/ipc/messages?channel=planning" 2>/dev/null) || resp=""
  if echo "$resp" | grep -q "broadcast test"; then
    tap_ok "broadcast messaging (planning channel)"
  else
    tap_fail "broadcast messaging - 'broadcast test' not in /api/ipc/messages?channel=planning"
  fi
}

# --- Test 6: Heartbeat ---
test_heartbeat() {
  bash "$SCRIPT_DIR/agent-heartbeat.sh" --name e2e-claude --task E2E-T1 2>/dev/null
  local agents_resp msg_resp
  agents_resp=$(curl "${CURL_OPTS[@]}" "${CONVERGIO_DAEMON_URL}/api/ipc/agents" 2>/dev/null) || agents_resp=""
  msg_resp=$(curl "${CURL_OPTS[@]}" "${CONVERGIO_DAEMON_URL}/api/ipc/messages?channel=heartbeat" 2>/dev/null) || msg_resp=""
  if echo "$agents_resp" | grep -q "e2e-claude" && echo "$msg_resp" | grep -qE "heartbeat|e2e-claude"; then
    tap_ok "heartbeat (agent updated + message on heartbeat channel)"
  else
    tap_fail "heartbeat - agent or heartbeat message not found"
  fi
}

# --- Test 7: Agent unregistration ---
test_unregister() {
  bash "$SCRIPT_DIR/agent-bridge.sh" --unregister --name e2e-claude 2>/dev/null
  bash "$SCRIPT_DIR/copilot-bridge.sh" --unregister --name e2e-copilot 2>/dev/null
  local resp
  resp=$(curl "${CURL_OPTS[@]}" "${CONVERGIO_DAEMON_URL}/api/ipc/agents" 2>/dev/null) || resp=""
  if echo "$resp" | grep -qE "e2e-claude|e2e-copilot"; then
    tap_fail "agent unregistration - test agents still present"
  else
    tap_ok "agent unregistration (both agents removed)"
  fi
}

# --- Test 8: Daemon-down resilience ---
test_daemon_down_resilience() {
  local exit_code=0
  CONVERGIO_DAEMON_URL=http://localhost:19999 \
    bash "$SCRIPT_DIR/agent-bridge.sh" --register --name e2e-offline --type claude 2>/dev/null || exit_code=$?
  if [[ "$exit_code" -eq 0 ]]; then
    tap_ok "daemon-down resilience (graceful failure, exit 0)"
  else
    tap_fail "daemon-down resilience - exit code $exit_code (expected 0)"
  fi
}

# --- Run tests ---
test_daemon_reachable

if [[ "$DAEMON_UP" == "true" ]]; then
  test_register_claude
  test_register_copilot
  test_messaging
  test_broadcast
  test_heartbeat
  test_unregister
else
  skip_remaining
fi

test_daemon_down_resilience

echo "1..${TEST_NUM}"
if [[ "$FAILURES" -gt 0 ]]; then
  exit 1
fi
exit 0
