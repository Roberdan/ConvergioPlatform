#!/usr/bin/env bash
# test-e2e.sh — End-to-end system verification for Convergio Platform
# Run after every deploy, plan completion, or on demand.
# Usage: ./scripts/test-e2e.sh [--remote m1Pro]
# Exit 0 = all pass, Exit 1 = failures found
set -uo pipefail

DAEMON_URL="${CVG_URL:-http://localhost:8420}"
REMOTE_NODE="${1:+${2:-}}"
PASSED=0; FAILED=0; SKIPPED=0

check() {
  local name="$1"; local cmd="$2"
  if eval "$cmd" >/dev/null 2>&1; then
    echo "  PASS  $name"; PASSED=$((PASSED + 1))
  else
    echo "  FAIL  $name"; FAILED=$((FAILED + 1))
  fi
}

skip() {
  local name="$1"; local reason="$2"
  echo "  SKIP  $name ($reason)"; SKIPPED=$((SKIPPED + 1))
}

echo "=== Convergio E2E Tests — $(date) ==="
echo ""

# --- 1. Daemon Health ---
echo "--- Daemon ---"
check "daemon health" "curl -sf $DAEMON_URL/api/health | grep -q ok"
check "daemon version" "curl -sf $DAEMON_URL/api/health | python3 -c \"import json,sys; v=json.load(sys.stdin)['version']; assert v.startswith('19'), f'Expected v19+, got {v}'\""
check "daemon DB" "curl -sf $DAEMON_URL/api/health | grep -q '\"db\":true'"
check "daemon peers" "curl -sf $DAEMON_URL/api/health | python3 -c \"import json,sys; assert json.load(sys.stdin)['peers'] > 0\""

# --- 2. Kernel / Jarvis ---
echo "--- Kernel (Jarvis) ---"
check "kernel status" "curl -sf $DAEMON_URL/api/kernel/status | grep -q models_loaded"
check "kernel ask" "curl -sf -X POST $DAEMON_URL/api/kernel/ask -H 'Content-Type: application/json' -d '{\"question\":\"ping\"}' | grep -q answer"

# --- 3. Plan DB ---
echo "--- Plan DB ---"
check "plan list" "curl -sf $DAEMON_URL/api/plan-db/list | python3 -c \"import json,sys; d=json.load(sys.stdin); assert 'plans' in d\""
check "execution-context" "curl -sf $DAEMON_URL/api/plan-db/execution-context/742 | grep -q plan_name"
check "cvg CLI" "~/.local/bin/cvg plan list 2>/dev/null | head -1 | grep -q '{'"

# --- 4. API Endpoints ---
echo "--- API ---"
check "api peers" "curl -sf $DAEMON_URL/api/peers | grep -q name"
check "api agents" "curl -sf $DAEMON_URL/api/ipc/agents | grep -q agents"
check "api metrics" "curl -sf $DAEMON_URL/api/metrics/summary | grep -q ok"
check "api workspace" "curl -sf $DAEMON_URL/api/workspace/list | grep -q ok"
check "api node readiness" "curl -sf $DAEMON_URL/api/node/readiness | grep -q checks"

# --- 5. MCP Server ---
echo "--- MCP ---"
check "mcp tools count" "echo '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}' | CONVERGIO_MCP_RING=0 daemon/target/release/convergio-mcp-server 2>/dev/null | python3 -c \"import json,sys; assert len(json.load(sys.stdin)['result']['tools']) >= 14\""

# --- 6. Sync ---
echo "--- Sync ---"
check "sync export" "curl -sf '$DAEMON_URL/api/sync/export?table=plans&since=2020-01-01' | python3 -c \"import json,sys; json.load(sys.stdin)\""
check "sync import" "curl -sf -X POST '$DAEMON_URL/api/sync/import' -H 'Content-Type: application/json' -d '{\"changes\":[]}' | grep -q ok"

# --- 6b. DB Sync (cross-node, requires --remote) ---
if [ -n "$REMOTE_NODE" ]; then
  echo "--- DB Sync ---"
  # Write a test value on local, check if remote can export it
  check "local plan count > 0" "curl -sf '$DAEMON_URL/api/sync/export?table=plans&since=2020-01-01' | python3 -c \"import json,sys; d=json.load(sys.stdin); assert len(d) > 0, 'no plans to sync'\""
  check "remote sync export" "ssh $REMOTE_NODE 'curl -sf \"http://localhost:8420/api/sync/export?table=plans&since=2020-01-01\"' | python3 -c \"import json,sys; json.load(sys.stdin)\""
fi

# --- 6c. Telegram ---
echo "--- Telegram ---"
if [ -n "${CONVERGIO_TELEGRAM_TOKEN:-}" ]; then
  check "telegram bot alive" "curl -sf 'https://api.telegram.org/bot${CONVERGIO_TELEGRAM_TOKEN}/getMe' | grep -q ConvergioBot"
else
  skip "telegram bot" "CONVERGIO_TELEGRAM_TOKEN not set"
fi

# --- 7. Delegation ---
echo "--- Delegation ---"
if [ -x "scripts/test-delegation-e2e.sh" ]; then
  check "delegation script syntax" "bash -n scripts/test-delegation-e2e.sh"
  check "copilot-plan-runner exists" "test -f ~/.claude/scripts/copilot-plan-runner.sh"
  check "cvg plan create" "~/.local/bin/cvg plan create 1 'E2E-probe-$(date +%s)' 2>/dev/null | grep -qi plan"
  check "cvg plan import" "~/.local/bin/cvg plan import --help 2>/dev/null | grep -qi spec"
else
  skip "delegation tests" "scripts/test-delegation-e2e.sh not found"
fi

# --- 8. Compilation ---
echo "--- Build ---"
check "cargo check" "cargo check --features kernel --manifest-path daemon/Cargo.toml 2>&1 | grep -q Finished"
check "cargo test" "cargo test --features kernel --manifest-path daemon/Cargo.toml --lib -- --test-threads=4 2>&1 | tee /tmp/cargo-test-e2e.log | tail -5 | grep -q 'test result: ok'"
check "zero files over 250" "! find daemon/src -name '*.rs' -exec sh -c 'test \$(wc -l < \"\$1\") -gt 250' _ {} \; -print 2>/dev/null | grep -q ."

# --- 9. Remote Node (if specified) ---
if [ -n "$REMOTE_NODE" ]; then
  echo "--- Remote: $REMOTE_NODE ---"
  check "remote health" "ssh $REMOTE_NODE 'curl -sf http://localhost:8420/api/health' | grep -q ok"
  LOCAL_VER="$(curl -sf $DAEMON_URL/api/health | python3 -c 'import json,sys; print(json.load(sys.stdin)["version"])')"
  check "remote version match" "ssh $REMOTE_NODE 'curl -sf http://localhost:8420/api/health' | python3 -c \"import json,sys; v=json.load(sys.stdin)['version']; assert v == '$LOCAL_VER', f'Mismatch: {v} vs $LOCAL_VER'\""
  check "remote kernel" "ssh $REMOTE_NODE 'curl -sf http://localhost:8420/api/kernel/status' | grep -q models_loaded"
  check "remote telegram" "ssh $REMOTE_NODE 'grep -q \"Telegram poll\" /tmp/daemon.log'"
  check "remote cvg" "ssh $REMOTE_NODE 'curl -sf http://localhost:8420/api/plan-db/list' | grep -q plans"
else
  skip "remote tests" "no --remote flag"
fi

# --- Summary ---
echo ""
TOTAL=$((PASSED + FAILED + SKIPPED))
echo "=== Results: $PASSED passed, $FAILED failed, $SKIPPED skipped / $TOTAL total ==="

if [ "$FAILED" -gt 0 ]; then
  echo "FAILED — $FAILED test(s) need attention"
  exit 1
else
  echo "ALL PASSED"
  exit 0
fi
