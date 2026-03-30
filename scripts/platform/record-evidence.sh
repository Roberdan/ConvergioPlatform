#!/usr/bin/env bash
# record-evidence.sh — post task evidence to daemon API.
# WHY: TestGate (status=submitted) requires at least one test_pass evidence row.
#
# Usage:
#   record-evidence.sh <task_db_id> <evidence_type> <command> <exit_code> [output_summary]
#
# evidence_type: test_pass | build_pass | lint_pass | curl_output
#
# Exit codes:
#   0 — evidence recorded
#   1 — argument error
#   2 — daemon API call failed
set -euo pipefail

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"

usage() {
  echo "Usage: $0 <task_db_id> <evidence_type> <command> <exit_code> [output_summary]" >&2
  echo "  evidence_type: test_pass | build_pass | lint_pass | curl_output" >&2
  exit 1
}

[ $# -lt 4 ] && usage

TASK_DB_ID="$1"
EVIDENCE_TYPE="$2"
COMMAND="$3"
EXIT_CODE="$4"
OUTPUT_SUMMARY="${5:-}"

# Validate numeric task_db_id
if ! echo "${TASK_DB_ID}" | grep -qE '^[0-9]+$'; then
  echo "ERROR: task_db_id must be numeric, got: ${TASK_DB_ID}" >&2
  exit 1
fi

# Validate evidence_type allowlist
case "${EVIDENCE_TYPE}" in
  test_pass|build_pass|lint_pass|curl_output) ;;
  *)
    echo "ERROR: unknown evidence_type '${EVIDENCE_TYPE}'" >&2
    echo "Valid: test_pass|build_pass|lint_pass|curl_output" >&2
    exit 1
    ;;
esac

PAYLOAD=$(printf '{"task_id":%d,"evidence_type":"%s","command":"%s","exit_code":%d,"output_summary":"%s"}' \
  "${TASK_DB_ID}" \
  "${EVIDENCE_TYPE}" \
  "$(echo "${COMMAND}" | sed 's/"/\\"/g')" \
  "${EXIT_CODE}" \
  "$(echo "${OUTPUT_SUMMARY}" | sed 's/"/\\"/g' | head -c 2000)")

RESPONSE=$(curl -sf -X POST \
  "${DAEMON_URL}/api/plan-db/task/evidence" \
  -H 'Content-Type: application/json' \
  -d "${PAYLOAD}" 2>&1) || {
  echo "ERROR: daemon API call failed: ${RESPONSE}" >&2
  exit 2
}

echo "${RESPONSE}"
echo "evidence recorded: task=${TASK_DB_ID} type=${EVIDENCE_TYPE} exit=${EXIT_CODE}"
