#!/usr/bin/env bash
set -euo pipefail

PEER="roberdandev@100.106.173.118"
MODE="all"
TIMEOUT_SECS=60
PROJECT_ID="convergio"
LOCAL_DAEMON_URL="${LOCAL_DAEMON_URL:-http://localhost:8420}"
REMOTE_DAEMON_URL="${REMOTE_DAEMON_URL:-http://localhost:8420}"
PLAN_ID=""
TASK_ID=""
TASK_STATUS="in_progress"
SKIP_PREFLIGHT=0

usage() {
  cat <<'EOF'
Usage: bash scripts/test-sync-two-node.sh --peer user@host --mode <mode> [options]

Modes:
  plan-m5-to-m1  Create plan locally, prove it appears on remote node within timeout.
  plan-m1-to-m5  Create plan remotely, prove it appears on local node within timeout.
  task-roundtrip Update one task status locally, prove remote observes same status.
  all            Run all modes in sequence (task-roundtrip needs --plan-id and --task-id).

Options:
  --peer <user@host>      Remote SSH target (default: roberdandev@100.106.173.118)
  --mode <mode>           One of: plan-m5-to-m1, plan-m1-to-m5, task-roundtrip, all
  --timeout <seconds>     Replication timeout per mode (default: 60)
  --project-id <id>       Project ID for probe plan creation (default: convergio)
  --plan-id <id>          Existing plan ID for task-roundtrip mode
  --task-id <id>          Existing task DB ID for task-roundtrip mode
  --task-status <status>  Target status for task-roundtrip (default: in_progress)
  --skip-preflight        Skip scripts/test-sync-preflight.sh
  -h, --help              Show this help text
EOF
}

log() { echo "[$(date -u +%H:%M:%S)] $*"; }
fail() { echo "❌ SYNC HARNESS FAILED: $*" >&2; exit 1; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "Missing required command '$1'"
}

run_cmd() {
  log "+ $*"
  "$@" || fail "Command failed: $*"
}

check_sync_status_payload() {
  local label="$1"
  local payload="$2"
  echo "$payload" | jq -e '.transport_mode == "daemon-http"' >/dev/null || \
    fail "$label /api/sync/status transport_mode is not daemon-http: $payload"
  echo "$payload" | jq -e '.fallback_policy == "manual-rsync-only"' >/dev/null || \
    fail "$label /api/sync/status fallback_policy is not manual-rsync-only: $payload"
}

fetch_local_sync_status() {
  curl -fsS "$LOCAL_DAEMON_URL/api/sync/status" || \
    fail "Local /api/sync/status unreachable at $LOCAL_DAEMON_URL"
}

fetch_remote_sync_status() {
  ssh -o BatchMode=yes -o ConnectTimeout=10 "$PEER" \
    "curl -fsS '$REMOTE_DAEMON_URL/api/sync/status'" || \
    fail "Remote /api/sync/status unreachable via $PEER at $REMOTE_DAEMON_URL"
}

check_daemon_first_policy() {
  log "Checking daemon-first policy on both nodes"
  local local_status remote_status
  local_status="$(fetch_local_sync_status)"
  remote_status="$(fetch_remote_sync_status)"
  check_sync_status_payload "Local" "$local_status"
  check_sync_status_payload "Remote" "$remote_status"
}

wait_for_remote_plan() {
  local plan_id="$1"
  local start now
  start="$(date +%s)"
  while true; do
    if ssh -o BatchMode=yes -o ConnectTimeout=10 "$PEER" \
      "curl -fsS '$REMOTE_DAEMON_URL/api/plan-db/json/$plan_id' | jq -e '.ok == true'" \
      >/dev/null 2>&1; then
      return 0
    fi
    now="$(date +%s)"
    if (( now - start >= TIMEOUT_SECS )); then
      fail "Plan $plan_id not visible on remote node within ${TIMEOUT_SECS}s"
    fi
    sleep 2
  done
}

wait_for_local_plan() {
  local plan_id="$1"
  local start now
  start="$(date +%s)"
  while true; do
    if curl -fsS "$LOCAL_DAEMON_URL/api/plan-db/json/$plan_id" | jq -e '.ok == true' >/dev/null 2>&1; then
      return 0
    fi
    now="$(date +%s)"
    if (( now - start >= TIMEOUT_SECS )); then
      fail "Plan $plan_id not visible on local node within ${TIMEOUT_SECS}s"
    fi
    sleep 2
  done
}

create_local_probe_plan() {
  local name="sync-probe-local-$(date -u +%Y%m%dT%H%M%SZ)"
  cvg plan create "$PROJECT_ID" "$name" | jq -er '.plan_id' || \
    fail "Unable to create local probe plan with cvg plan create"
}

create_remote_probe_plan() {
  local name="sync-probe-remote-$(date -u +%Y%m%dT%H%M%SZ)"
  ssh -o BatchMode=yes -o ConnectTimeout=10 "$PEER" \
    "cvg plan create '$PROJECT_ID' '$name' | jq -er '.plan_id'" || \
    fail "Unable to create remote probe plan on $PEER"
}

run_plan_m5_to_m1() {
  check_daemon_first_policy
  local plan_id
  plan_id="$(create_local_probe_plan)"
  log "Created local probe plan: $plan_id"
  wait_for_remote_plan "$plan_id"
  log "✅ plan-m5-to-m1 replicated plan $plan_id within ${TIMEOUT_SECS}s"
}

run_plan_m1_to_m5() {
  check_daemon_first_policy
  local plan_id
  plan_id="$(create_remote_probe_plan)"
  log "Created remote probe plan: $plan_id"
  wait_for_local_plan "$plan_id"
  log "✅ plan-m1-to-m5 replicated plan $plan_id within ${TIMEOUT_SECS}s"
}

wait_for_remote_task_status() {
  local plan_id="$1"
  local task_id="$2"
  local expected="$3"
  local start now
  start="$(date +%s)"
  while true; do
    if ssh -o BatchMode=yes -o ConnectTimeout=10 "$PEER" \
      "curl -fsS '$REMOTE_DAEMON_URL/api/plan-db/json/$plan_id' | jq -e '.tasks[] | select(.id == $task_id and .status == \"$expected\")'" \
      >/dev/null 2>&1; then
      return 0
    fi
    now="$(date +%s)"
    if (( now - start >= TIMEOUT_SECS )); then
      fail "Task $task_id in plan $plan_id did not reach status '$expected' on remote within ${TIMEOUT_SECS}s"
    fi
    sleep 2
  done
}

run_task_roundtrip() {
  check_daemon_first_policy
  [[ -n "$PLAN_ID" ]] || fail "--plan-id is required for task-roundtrip mode"
  [[ -n "$TASK_ID" ]] || fail "--task-id is required for task-roundtrip mode"
  run_cmd cvg task update "$TASK_ID" "$TASK_STATUS" --summary "sync harness probe"
  wait_for_remote_task_status "$PLAN_ID" "$TASK_ID" "$TASK_STATUS"
  log "✅ task-roundtrip replicated task $TASK_ID status '$TASK_STATUS' within ${TIMEOUT_SECS}s"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --peer) PEER="${2:-}"; shift 2 ;;
    --mode) MODE="${2:-}"; shift 2 ;;
    --timeout) TIMEOUT_SECS="${2:-}"; shift 2 ;;
    --project-id) PROJECT_ID="${2:-}"; shift 2 ;;
    --plan-id) PLAN_ID="${2:-}"; shift 2 ;;
    --task-id) TASK_ID="${2:-}"; shift 2 ;;
    --task-status) TASK_STATUS="${2:-}"; shift 2 ;;
    --skip-preflight) SKIP_PREFLIGHT=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fail "Unknown argument: $1" ;;
  esac
done

for cmd in bash curl jq ssh cvg; do
  require_cmd "$cmd"
done
[[ "$TIMEOUT_SECS" =~ ^[0-9]+$ ]] || fail "--timeout must be an integer"

if (( SKIP_PREFLIGHT == 0 )); then
  run_cmd bash scripts/test-sync-preflight.sh --peer "$PEER"
fi

case "$MODE" in
  plan-m5-to-m1) run_plan_m5_to_m1 ;;
  plan-m1-to-m5) run_plan_m1_to_m5 ;;
  task-roundtrip) run_task_roundtrip ;;
  all)
    run_plan_m5_to_m1
    run_plan_m1_to_m5
    run_task_roundtrip
    ;;
  *) fail "Invalid --mode '$MODE'" ;;
esac
