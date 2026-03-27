#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAUDE_HOME="${CLAUDE_HOME:-$HOME/.claude}"
DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"
DRY_RUN=false

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 2; }

usage() {
  cat <<'USAGE'
Usage: mesh-normalize-hosts.sh [--dry-run]

One-time migration that normalizes plans.execution_host and tasks.executor_host
using canonical peer names from peers.conf.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=true; shift ;;
    --help|-h) usage; exit 0 ;;
    *)
      echo "ERROR: Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! curl -sf "${DAEMON_URL}/api/health" &>/dev/null; then
  echo "ERROR: Daemon not reachable at ${DAEMON_URL}" >&2
  exit 1
fi

# shellcheck source=scripts/lib/peers.sh
source "$SCRIPT_DIR/lib/peers.sh"
peers_load

sql_quote() {
  local value="$1"
  [[ "$value" != *$'\n'* && "$value" != *$'\r'* ]] || return 1
  printf "%s" "$value" | sed "s/'/''/g"
}

is_valid_hostname() {
  local value="$1"
  [[ -n "$value" && ${#value} -le 253 ]] || return 1
  [[ "$value" =~ ^[A-Za-z0-9._-]+$ ]]
}

trim() {
  local s="$1"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf "%s" "$s"
}

normalize_key() {
  printf "%s" "$1" | tr '[:upper:]' '[:lower:]' | tr -cd '[:alnum:]'
}

SQL_FILE="$(mktemp)"
trap 'rm -f "$SQL_FILE"' EXIT

{
  echo "BEGIN IMMEDIATE;"
  echo "CREATE TEMP TABLE host_map (variant TEXT PRIMARY KEY, canonical TEXT NOT NULL);"
} >"$SQL_FILE"

add_variant() {
  local canonical="$1"
  local variant_raw="$2"
  local variant
  variant="$(trim "$variant_raw")"
  [[ -z "$canonical" || -z "$variant" ]] && return 0
  is_valid_hostname "$canonical" || return 0
  is_valid_hostname "$variant" || return 0

  local q_canonical q_variant
  q_canonical="$(sql_quote "$canonical")" || return 0
  q_variant="$(sql_quote "$variant")" || return 0
  printf '%s\n' "INSERT OR IGNORE INTO host_map(variant, canonical) VALUES ('$q_variant', '$q_canonical');" >>"$SQL_FILE"
}

resolve_peer_for_variant() {
  local variant="$1"
  local variant_key matched="" peer candidate candidate_key
  variant_key="$(normalize_key "$variant")"
  [[ -z "$variant_key" ]] && return 1

  for peer in $_PEERS_ALL; do
    for candidate in \
      "$peer" \
      "$(peers_get "$peer" ssh_alias 2>/dev/null || true)" \
      "$(peers_get "$peer" dns_name 2>/dev/null || true)"; do
      candidate_key="$(normalize_key "$candidate")"
      [[ -z "$candidate_key" ]] && continue
      if [[ "$variant_key" == *"$candidate_key"* || "$candidate_key" == *"$variant_key"* ]]; then
        if [[ -n "$matched" && "$matched" != "$peer" ]]; then
          return 1
        fi
        matched="$peer"
        break
      fi
    done
  done

  [[ -n "$matched" ]] || return 1
  printf "%s" "$matched"
}

# Build peer variants -> canonical mapping for each section in peers.conf
for peer in $_PEERS_ALL; do
  add_variant "$peer" "$peer"
  add_variant "$peer" "$(peers_get "$peer" ssh_alias 2>/dev/null || true)"
  add_variant "$peer" "$(peers_get "$peer" dns_name 2>/dev/null || true)"
done

# Local machine historical variants -> peers_self canonical name
SELF_PEER="$(peers_self || true)"
if [[ -n "$SELF_PEER" ]]; then
  HN_SHORT="$(hostname -s 2>/dev/null || true)"
  HN_FULL="$(hostname 2>/dev/null || true)"
  LOCAL_HOSTNAME="$(scutil --get LocalHostName 2>/dev/null || true)"
  COMPUTER_NAME="$(scutil --get ComputerName 2>/dev/null | tr -d "'" || true)"

  add_variant "$SELF_PEER" "$HN_SHORT"
  add_variant "$SELF_PEER" "$HN_FULL"
  add_variant "$SELF_PEER" "$LOCAL_HOSTNAME"
  add_variant "$SELF_PEER" "$COMPUTER_NAME"

  # Common macOS DNS-style variants seen historically
  [[ -n "$HN_SHORT" ]] && add_variant "$SELF_PEER" "${HN_SHORT}.lan"
  [[ -n "$LOCAL_HOSTNAME" ]] && add_variant "$SELF_PEER" "${LOCAL_HOSTNAME}.lan"
  [[ -n "$COMPUTER_NAME" ]] && add_variant "$SELF_PEER" "${COMPUTER_NAME}.lan"
fi

# Fuzzy pass: map historical DB host variants to canonical peers when match is unique.
# Fetch distinct hosts from plans and tasks via daemon API
ALL_PLANS_DATA=$(curl -sf "${DAEMON_URL}/api/plan-db/plans" 2>/dev/null || echo '[]')
HOST_VARIANTS=$(echo "$ALL_PLANS_DATA" | jq -r '
  [.[].execution_host // empty] | unique | .[]' 2>/dev/null || echo "")

# Also gather task executor_host from active plans
ACTIVE_IDS=$(echo "$ALL_PLANS_DATA" | jq -r '.[].id' 2>/dev/null || echo "")
for pid in $ACTIVE_IDS; do
  PLAN_DETAIL=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${pid}" 2>/dev/null || echo '{}')
  TASK_HOSTS=$(echo "$PLAN_DETAIL" | jq -r '[.tasks[]?.executor_host // empty] | unique | .[]' 2>/dev/null || echo "")
  [[ -n "$TASK_HOSTS" ]] && HOST_VARIANTS=$(printf '%s\n%s' "$HOST_VARIANTS" "$TASK_HOSTS")
done
HOST_VARIANTS=$(echo "$HOST_VARIANTS" | sort -u)

while IFS= read -r host_variant; do
  host_variant="$(trim "$host_variant")"
  [[ -z "$host_variant" ]] && continue
  canonical="$(resolve_peer_for_variant "$host_variant" || true)"
  [[ -z "$canonical" ]] && continue
  add_variant "$canonical" "$host_variant"
done <<<"$HOST_VARIANTS"

# Build variant->canonical mapping from SQL_FILE for use in API calls
# Parse the INSERT statements we wrote to SQL_FILE to build a jq-friendly map
VARIANT_MAP=$(grep "INSERT OR IGNORE INTO host_map" "$SQL_FILE" | \
  sed "s/.*VALUES ('\(.*\)', '\(.*\)');/\1|\2/" | sort -u)

if $DRY_RUN; then
  echo "--- Host mapping (variant -> canonical) ---"
  echo "$VARIANT_MAP" | while IFS='|' read -r variant canonical; do
    echo "  $variant -> $canonical"
  done
  echo "--- Would normalize plans, tasks, and clean test heartbeats ---"
  exit 0
fi

plans_count=0
tasks_count=0

# Normalize plan execution_host via API
while IFS='|' read -r variant canonical; do
  [[ -z "$variant" || -z "$canonical" || "$variant" == "$canonical" ]] && continue
  # Find plans with this variant and update them
  MATCHING_PLANS=$(echo "$ALL_PLANS_DATA" | jq -r --arg v "$variant" \
    '[.[] | select(.execution_host == $v and .status != "doing")] | .[].id' 2>/dev/null || echo "")
  for pid in $MATCHING_PLANS; do
    [[ -z "$pid" ]] && continue
    curl -sf -X POST "${DAEMON_URL}/api/plan-db/plan/update-host" \
      -H 'Content-Type: application/json' \
      -d "{\"plan_id\":${pid},\"execution_host\":\"${canonical}\"}" 2>/dev/null && \
      plans_count=$((plans_count + 1)) || true
  done
done <<<"$VARIANT_MAP"

# Normalize task executor_host via API
for pid in $ACTIVE_IDS; do
  PLAN_DETAIL=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${pid}" 2>/dev/null || echo '{}')
  while IFS='|' read -r variant canonical; do
    [[ -z "$variant" || -z "$canonical" || "$variant" == "$canonical" ]] && continue
    MATCHING_TASKS=$(echo "$PLAN_DETAIL" | jq -r --arg v "$variant" \
      '[.tasks[]? | select(.executor_host == $v)] | .[].id' 2>/dev/null || echo "")
    for tid in $MATCHING_TASKS; do
      [[ -z "$tid" ]] && continue
      curl -sf -X POST "${DAEMON_URL}/api/plan-db/task/update-host" \
        -H 'Content-Type: application/json' \
        -d "{\"task_id\":${tid},\"executor_host\":\"${canonical}\"}" 2>/dev/null && \
        tasks_count=$((tasks_count + 1)) || true
    done
  done <<<"$VARIANT_MAP"
done

# Clean test heartbeats via API
heartbeats_count=0
curl -sf -X POST "${DAEMON_URL}/api/heartbeat/cleanup-test" \
  -H 'Content-Type: application/json' 2>/dev/null && heartbeats_count=1 || true

echo "Normalized ${plans_count} plans, ${tasks_count} tasks. Cleaned test heartbeats (${heartbeats_count} batch ops)."
