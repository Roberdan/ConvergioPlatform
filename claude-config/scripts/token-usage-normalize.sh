#!/usr/bin/env bash
set -euo pipefail

DAEMON_API="http://localhost:8420"
MODE="${1:---check}"

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

if [[ "$MODE" == "--check" ]]; then
  # Check for missing token normalization via daemon metrics
  _metrics="$(curl -sf "${DAEMON_API}/api/metrics/summary" 2>/dev/null || echo '{}')"
  echo "$_metrics" | jq '{
    missing_rows: (.token_normalization_missing // 0),
    zero_token_tasks: (.zero_token_tasks // 0)
  }' 2>/dev/null || echo '{"missing_rows":0,"zero_token_tasks":0}'
  exit 0
fi

if [[ "$MODE" != "--apply" ]]; then
  echo "Usage: $0 [--check|--apply]" >&2
  exit 2
fi

# Trigger token normalization via daemon API
_result="$(curl -sf -X POST "${DAEMON_API}/api/tracking/tokens" \
  -H 'Content-Type: application/json' \
  -d '{"action":"normalize"}' 2>/dev/null || echo '{}')"

echo "$_result" | jq '.' 2>/dev/null || echo "$_result"

# Show updated status
_metrics="$(curl -sf "${DAEMON_API}/api/metrics/summary" 2>/dev/null || echo '{}')"
echo "$_metrics" | jq '{
  missing_rows: (.token_normalization_missing // 0),
  zero_token_tasks: (.zero_token_tasks // 0)
}' 2>/dev/null || echo '{"missing_rows":0,"zero_token_tasks":0}'
