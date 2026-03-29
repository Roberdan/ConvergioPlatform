#!/usr/bin/env bash
# test-delegation-e2e.sh — Full delegation E2E: create micro plan, import,
# start, run copilot-plan-runner, verify completion.
# Usage: ./scripts/test-delegation-e2e.sh [--timeout 300]
# Exit 0 = pass, Exit 1 = fail
set -uo pipefail
trap cleanup EXIT

DAEMON_URL="${CVG_URL:-http://localhost:8420}"
CVG="${CVG_BIN:-$HOME/.local/bin/cvg}"
RUNNER="${RUNNER_BIN:-$HOME/.claude/scripts/copilot-plan-runner.sh}"
TIMEOUT="${2:-300}"
TMPDIR_E2E=""
TEST_FILE=""
PLAN_ID=""

# --- Helpers ---

cleanup() {
  if [ -n "$TMPDIR_E2E" ] && [ -d "$TMPDIR_E2E" ]; then
    rm -rf "$TMPDIR_E2E"
  fi
  if [ -n "$TEST_FILE" ] && [ -f "$TEST_FILE" ]; then
    rm -f "$TEST_FILE"
  fi
  # Cancel plan if still running (best-effort)
  if [ -n "$PLAN_ID" ]; then
    local status
    status="$(get_plan_status)"
    if [ "$status" = "doing" ] || [ "$status" = "todo" ]; then
      "$CVG" plan cancel "$PLAN_ID" "E2E test cleanup" 2>/dev/null || true
    fi
  fi
}

fail() { echo "FAIL: $1" >&2; exit 1; }
info() { echo "[E2E] $1"; }

get_plan_status() {
  curl -sf "${DAEMON_URL}/api/plan-db/json/${PLAN_ID}" 2>/dev/null \
    | python3 -c "import json,sys; print(json.load(sys.stdin).get('status','unknown'))" 2>/dev/null \
    || echo "unknown"
}

get_task_count_by_status() {
  local status="$1"
  curl -sf "${DAEMON_URL}/api/plan-db/json/${PLAN_ID}" 2>/dev/null \
    | python3 -c "
import json,sys
d=json.load(sys.stdin)
print(sum(1 for t in d.get('tasks',[]) if t.get('status')=='$status'))
" 2>/dev/null || echo "0"
}

# --- Pre-flight ---

info "Pre-flight checks..."
curl -sf "${DAEMON_URL}/api/health" >/dev/null 2>&1 \
  || fail "Daemon not running at ${DAEMON_URL}"
command -v python3 >/dev/null 2>&1 || fail "python3 not found"
[ -x "$CVG" ] || fail "cvg CLI not found at $CVG"
[ -f "$RUNNER" ] || fail "copilot-plan-runner.sh not found at $RUNNER"
(command -v claude >/dev/null 2>&1 || command -v copilot >/dev/null 2>&1) \
  || fail "Neither claude nor copilot CLI found"

# --- Step 1: Create micro test plan spec ---

TMPDIR_E2E="$(mktemp -d)"
TEST_FILE="/tmp/convergio-delegation-e2e-proof-$(date +%s).txt"
SPEC_FILE="${TMPDIR_E2E}/delegation-e2e-spec.yaml"

info "Creating micro plan spec..."
cat > "$SPEC_FILE" << YAML
waves:
  - id: "W1"
    name: "Delegation E2E"
    depends_on: null
    estimated_hours: 1
    tasks:
      - id: "T1-01"
        title: "Create proof file for delegation E2E test"
        type: feature
        priority: P2
        description: |
          Create the file ${TEST_FILE} with content 'delegation-e2e-ok'.
          This is an automated E2E test. Just create the file, nothing else.
        model: claude-sonnet-4-6
        files:
          - "${TEST_FILE}"
        verify:
          - "test -f ${TEST_FILE}"
          - "grep -q delegation-e2e-ok ${TEST_FILE}"
        test_criteria: "File ${TEST_FILE} exists and contains 'delegation-e2e-ok'"
      - id: "T1-02"
        title: "Verify proof file exists"
        type: test
        priority: P2
        description: |
          Verify that ${TEST_FILE} exists and contains 'delegation-e2e-ok'.
          Run: cat ${TEST_FILE} and confirm content.
        model: claude-sonnet-4-6
        files:
          - "${TEST_FILE}"
        verify:
          - "grep -q delegation-e2e-ok ${TEST_FILE}"
        test_criteria: "File verified successfully"
YAML

# --- Step 2: Create plan in DB ---

info "Creating plan in DB..."
CREATE_OUT="$("$CVG" plan create 1 "E2E Delegation Test $(date +%H%M%S)" 2>&1)"
PLAN_ID="$(echo "$CREATE_OUT" | python3 -c "
import sys, re
text = sys.stdin.read()
m = re.search(r'[Pp]lan\s+#?(\d+)', text) or re.search(r'(\d+)', text)
print(m.group(1) if m else '')
" 2>/dev/null)"
[ -n "$PLAN_ID" ] || fail "Could not extract plan ID from: $CREATE_OUT"
info "Plan created: #${PLAN_ID}"

# --- Step 3: Import spec ---

info "Importing spec into plan #${PLAN_ID}..."
IMPORT_OUT="$("$CVG" plan import "$PLAN_ID" "$SPEC_FILE" 2>&1)"
echo "$IMPORT_OUT" | grep -qi "error" && fail "Import failed: $IMPORT_OUT"
info "Spec imported successfully"

# --- Step 4: Start plan ---

info "Starting plan #${PLAN_ID}..."
"$CVG" plan start "$PLAN_ID" 2>&1 || fail "Failed to start plan"
info "Plan started"

# --- Step 5: Launch copilot-plan-runner with timeout ---

info "Launching copilot-plan-runner.sh (timeout: ${TIMEOUT}s)..."
RUNNER_LOG="${TMPDIR_E2E}/runner.log"

# Run in background with timeout
timeout "$TIMEOUT" bash "$RUNNER" "$PLAN_ID" > "$RUNNER_LOG" 2>&1 &
RUNNER_PID=$!

# Poll for completion (check every 15s)
ELAPSED=0
POLL_INTERVAL=15
while [ "$ELAPSED" -lt "$TIMEOUT" ]; do
  sleep "$POLL_INTERVAL"
  ELAPSED=$((ELAPSED + POLL_INTERVAL))

  STATUS="$(get_plan_status)"
  DONE_COUNT="$(get_task_count_by_status done)"
  SUBMITTED_COUNT="$(get_task_count_by_status submitted)"
  info "  ${ELAPSED}s — status: ${STATUS}, done: ${DONE_COUNT}, submitted: ${SUBMITTED_COUNT}"

  if [ "$STATUS" = "done" ] || [ "$STATUS" = "completed" ]; then
    info "Plan completed!"
    break
  fi
  if [ "$STATUS" = "cancelled" ]; then
    fail "Plan was cancelled unexpectedly"
  fi

  # Check if runner is still alive
  if ! kill -0 "$RUNNER_PID" 2>/dev/null; then
    info "Runner process exited, checking final status..."
    break
  fi
done

# Kill runner if still running
kill "$RUNNER_PID" 2>/dev/null || true
wait "$RUNNER_PID" 2>/dev/null || true

# --- Step 6: Verify results ---

info "=== Verification ==="
FAILURES=0

# 6a: Plan status
FINAL_STATUS="$(get_plan_status)"
if [ "$FINAL_STATUS" = "done" ] || [ "$FINAL_STATUS" = "completed" ]; then
  info "  PASS  Plan status: ${FINAL_STATUS}"
else
  echo "  FAIL  Plan status: ${FINAL_STATUS} (expected done/completed)"
  FAILURES=$((FAILURES + 1))
fi

# 6b: Task statuses (at least submitted)
DONE_TASKS="$(get_task_count_by_status done)"
SUBMITTED_TASKS="$(get_task_count_by_status submitted)"
COMPLETED_TOTAL=$((DONE_TASKS + SUBMITTED_TASKS))
if [ "$COMPLETED_TOTAL" -ge 2 ]; then
  info "  PASS  Tasks done/submitted: ${COMPLETED_TOTAL}/2"
else
  echo "  FAIL  Tasks done/submitted: ${COMPLETED_TOTAL}/2"
  FAILURES=$((FAILURES + 1))
fi

# 6c: Proof file exists
if [ -f "$TEST_FILE" ] && grep -q "delegation-e2e-ok" "$TEST_FILE" 2>/dev/null; then
  info "  PASS  Proof file exists with correct content"
else
  echo "  FAIL  Proof file missing or wrong content: ${TEST_FILE}"
  FAILURES=$((FAILURES + 1))
fi

# 6d: Runner log shows activity
if [ -f "$RUNNER_LOG" ] && [ -s "$RUNNER_LOG" ]; then
  info "  PASS  Runner log has output ($(wc -l < "$RUNNER_LOG") lines)"
else
  echo "  FAIL  Runner log empty or missing"
  FAILURES=$((FAILURES + 1))
fi

# --- Summary ---

echo ""
if [ "$FAILURES" -eq 0 ]; then
  info "=== DELEGATION E2E: ALL PASSED ==="
  exit 0
else
  info "=== DELEGATION E2E: ${FAILURES} FAILURE(S) ==="
  info "Runner log: ${RUNNER_LOG}"
  info "Plan: cvg plan tree ${PLAN_ID}"
  exit 1
fi
