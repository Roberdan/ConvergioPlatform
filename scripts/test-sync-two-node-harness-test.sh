#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HARNESS="$ROOT_DIR/scripts/test-sync-two-node.sh"

[[ -f "$HARNESS" ]] || {
  echo "❌ missing harness script at $HARNESS" >&2
  exit 1
}

OUTPUT="$(
  LOCAL_DAEMON_URL="http://127.0.0.1:1" \
  bash "$HARNESS" \
    --mode plan-m5-to-m1 \
    --peer roberdandev@100.106.173.118 \
    --timeout 2 \
    --skip-preflight \
    2>&1 || true
)"

echo "$OUTPUT" | grep -q "SYNC HARNESS FAILED" || {
  echo "❌ expected fail-loud marker not found in harness output" >&2
  echo "$OUTPUT" >&2
  exit 1
}

echo "$OUTPUT" | grep -q "/api/sync/status" || {
  echo "❌ expected daemon-first HTTP diagnostic not found" >&2
  echo "$OUTPUT" >&2
  exit 1
}

echo "✅ harness fail-loud test passed"
