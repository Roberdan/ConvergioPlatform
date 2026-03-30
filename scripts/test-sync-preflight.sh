#!/usr/bin/env bash
set -euo pipefail

PEER="roberdandev@100.106.173.118"
LOCAL_DAEMON_URL="${LOCAL_DAEMON_URL:-http://localhost:8420}"
REMOTE_DAEMON_URL="${REMOTE_DAEMON_URL:-http://localhost:8420}"
HEALTH_PATH="${HEALTH_PATH:-/api/health}"

usage() {
  cat <<'EOF'
Usage: bash scripts/test-sync-preflight.sh [--peer user@host]

Checks:
  1. Required local commands are available.
  2. Local daemon health endpoint returns {"ok":true}.
  3. SSH reachability to peer.
  4. Required remote commands are available.
  5. Remote daemon health endpoint returns {"ok":true}.
EOF
}

fail() {
  echo "❌ PRECHECK FAILED: $*" >&2
  exit 1
}

check_cmd() {
  local cmd="$1"
  command -v "$cmd" >/dev/null 2>&1 || fail "Missing required command '$cmd'. Install it and retry."
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --peer)
      [[ $# -ge 2 ]] || fail "Missing value for --peer"
      PEER="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "Unknown argument: $1"
      ;;
  esac
done

echo "== Daemon Sync Preflight =="
echo "Peer: $PEER"
echo "Local daemon: $LOCAL_DAEMON_URL$HEALTH_PATH"
echo "Remote daemon: $REMOTE_DAEMON_URL$HEALTH_PATH"

echo "-> Checking required local commands"
for cmd in bash curl jq ssh cvg; do
  check_cmd "$cmd"
done

echo "-> Checking local daemon health"
LOCAL_HEALTH="$(curl -fsS "$LOCAL_DAEMON_URL$HEALTH_PATH")" || fail "Local daemon is unreachable at $LOCAL_DAEMON_URL$HEALTH_PATH"
echo "$LOCAL_HEALTH" | jq -e '.ok == true' >/dev/null || fail "Local daemon health check returned non-ok payload: $LOCAL_HEALTH"

echo "-> Checking SSH reachability to $PEER"
ssh -o BatchMode=yes -o ConnectTimeout=10 "$PEER" "echo ok" >/dev/null 2>&1 || \
  fail "Cannot SSH to $PEER. Verify VPN/Tailscale, host key trust, and SSH keys."

echo "-> Checking required remote commands"
ssh -o BatchMode=yes -o ConnectTimeout=10 "$PEER" "command -v bash && command -v curl && command -v jq" >/dev/null 2>&1 || \
  fail "Remote node missing required commands (bash/curl/jq) or command check failed."

echo "-> Checking remote daemon health"
REMOTE_HEALTH="$(ssh -o BatchMode=yes -o ConnectTimeout=10 "$PEER" "curl -fsS '$REMOTE_DAEMON_URL$HEALTH_PATH'")" || \
  fail "Remote daemon is unreachable at $PEER:$REMOTE_DAEMON_URL$HEALTH_PATH"
echo "$REMOTE_HEALTH" | jq -e '.ok == true' >/dev/null || fail "Remote daemon health check returned non-ok payload: $REMOTE_HEALTH"

echo "✅ Preflight checks passed. Environment ready for daemon-first sync verification."
