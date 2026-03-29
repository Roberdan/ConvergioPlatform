#!/usr/bin/env bash
set -euo pipefail

# Live two-node replication SLA test.
# Proves: plan created on M5Max replicates to M1 Pro within 60 seconds.
# Requires: both daemons running, SSH access, preflight passing.

PEER="${PEER:-roberdandev@100.106.173.118}"
LOCAL_URL="${LOCAL_DAEMON_URL:-http://localhost:8420}"
REMOTE_URL="${REMOTE_DAEMON_URL:-http://localhost:8420}"
SLA_SECS="${SLA_SECS:-60}"
POLL_INTERVAL=2
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

fail() {
  echo "FAIL: $*" >&2
  echo "DIAGNOSTIC: local_url=$LOCAL_URL remote_url=$REMOTE_URL peer=$PEER" >&2
  exit 1
}

info() { echo "-- $*"; }

usage() {
  cat <<'EOF'
Usage: bash scripts/test-sync-replication-sla.sh [--peer user@host] [--sla 60]

Proves plan replication from local (M5Max) to remote (M1 Pro) within SLA.
Steps:
  1. Run preflight checks
  2. Create a unique plan on local daemon
  3. Wait for background sync to replicate
  4. Poll remote daemon until plan appears or SLA expires
  5. Report timing and pass/fail
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --peer)  PEER="$2"; shift 2 ;;
    --sla)   SLA_SECS="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) fail "Unknown argument: $1" ;;
  esac
done

info "=== Sync Replication SLA Test ==="
info "Local:  $LOCAL_URL"
info "Remote: $REMOTE_URL (via $PEER)"
info "SLA:    ${SLA_SECS}s"

# Step 1: Preflight
info "Running preflight..."
if [[ -f "$SCRIPT_DIR/test-sync-preflight.sh" ]]; then
  bash "$SCRIPT_DIR/test-sync-preflight.sh" --peer "$PEER" || \
    fail "Preflight failed — fix environment before running SLA test"
else
  fail "Preflight script not found at $SCRIPT_DIR/test-sync-preflight.sh"
fi

# Step 2: Create unique plan on local daemon
PLAN_NAME="sla-test-$(date +%s)-$$"
info "Creating plan '$PLAN_NAME' on local daemon..."
CREATE_RESP=$(curl -fsS "$LOCAL_URL/api/plan-db/create" \
  -H 'Content-Type: application/json' \
  -d "{\"project_id\":\"convergio\",\"name\":\"$PLAN_NAME\",\"description\":\"SLA replication test\"}" \
  2>&1) || fail "Plan creation failed: $CREATE_RESP"

PLAN_ID=$(echo "$CREATE_RESP" | jq -r '.plan_id // .id // empty')
[[ -n "$PLAN_ID" ]] || fail "Cannot extract plan_id from response: $CREATE_RESP"
info "Created plan id=$PLAN_ID name=$PLAN_NAME"

# Step 3: Poll remote for plan appearance
START_TS=$(date +%s)
info "Polling remote for plan replication (SLA=${SLA_SECS}s)..."

FOUND=false
while true; do
  NOW=$(date +%s)
  ELAPSED=$((NOW - START_TS))

  if [[ $ELAPSED -ge $SLA_SECS ]]; then
    break
  fi

  REMOTE_PLANS=$(ssh -o BatchMode=yes -o ConnectTimeout=10 "$PEER" \
    "curl -fsS '$REMOTE_URL/api/sync/export?table=plans'" 2>/dev/null) || {
    info "  [${ELAPSED}s] remote fetch failed, retrying..."
    sleep "$POLL_INTERVAL"
    continue
  }

  if echo "$REMOTE_PLANS" | jq -e \
    ".changes[] | select(.pk == $PLAN_ID)" >/dev/null 2>&1; then
    FOUND=true
    break
  fi

  # Also check direct DB query via plan-db API
  REMOTE_PLAN=$(ssh -o BatchMode=yes -o ConnectTimeout=10 "$PEER" \
    "curl -fsS '$REMOTE_URL/api/plan-db/$PLAN_ID'" 2>/dev/null) || true

  if echo "$REMOTE_PLAN" | jq -e ".name == \"$PLAN_NAME\"" \
    >/dev/null 2>&1; then
    FOUND=true
    break
  fi

  info "  [${ELAPSED}s] plan not yet on remote, waiting..."
  sleep "$POLL_INTERVAL"
done

END_TS=$(date +%s)
TOTAL_ELAPSED=$((END_TS - START_TS))

# Step 4: Report
echo ""
echo "=== REPLICATION SLA RESULT ==="
echo "Plan ID:     $PLAN_ID"
echo "Plan Name:   $PLAN_NAME"
echo "Elapsed:     ${TOTAL_ELAPSED}s"
echo "SLA Target:  ${SLA_SECS}s"

if [[ "$FOUND" == "true" ]]; then
  echo "Status:      PASS"
  echo "Plan replicated in ${TOTAL_ELAPSED}s (within ${SLA_SECS}s SLA)"
  exit 0
else
  echo "Status:      FAIL"
  echo ""
  echo "=== FAIL-LOUD DIAGNOSTICS ==="
  info "Local sync status:"
  curl -fsS "$LOCAL_URL/api/sync/status" 2>&1 | jq . || echo "(unavailable)"
  info "Remote sync status:"
  ssh -o BatchMode=yes "$PEER" \
    "curl -fsS '$REMOTE_URL/api/sync/status'" 2>&1 | jq . || echo "(unavailable)"
  info "Local plan verify:"
  curl -fsS "$LOCAL_URL/api/sync/export?table=plans" 2>&1 | \
    jq ".changes[] | select(.pk == $PLAN_ID)" || echo "(not found)"
  fail "Plan $PLAN_ID did NOT replicate within ${SLA_SECS}s"
fi
