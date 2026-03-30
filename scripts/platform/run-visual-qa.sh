#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VQA_DIR="$ROOT/scripts/platform/visual-qa"

BASE_URL=""
STATIC_DIR=""
ROUTE="/"
READY_SELECTOR=""
EXPECTED_TITLE=""
SNAPSHOT_NAME="visual-qa-page"
PORT="4173"
UPDATE_SNAPSHOTS=0
HEADED=0
EXTRA_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base-url)
      BASE_URL="${2:?missing value for --base-url}"
      shift 2
      ;;
    --static-dir)
      STATIC_DIR="${2:?missing value for --static-dir}"
      shift 2
      ;;
    --route)
      ROUTE="${2:?missing value for --route}"
      shift 2
      ;;
    --ready-selector)
      READY_SELECTOR="${2:?missing value for --ready-selector}"
      shift 2
      ;;
    --expected-title)
      EXPECTED_TITLE="${2:?missing value for --expected-title}"
      shift 2
      ;;
    --snapshot-name)
      SNAPSHOT_NAME="${2:?missing value for --snapshot-name}"
      shift 2
      ;;
    --port)
      PORT="${2:?missing value for --port}"
      shift 2
      ;;
    --update-snapshots)
      UPDATE_SNAPSHOTS=1
      shift
      ;;
    --headed)
      HEADED=1
      shift
      ;;
    --)
      shift
      EXTRA_ARGS+=("$@")
      break
      ;;
    *)
      EXTRA_ARGS+=("$1")
      shift
      ;;
  esac
done

if [[ -z "$BASE_URL" && -z "$STATIC_DIR" ]]; then
  echo "error: provide --base-url or --static-dir" >&2
  exit 1
fi

if [[ -n "$BASE_URL" && -n "$STATIC_DIR" ]]; then
  echo "error: use either --base-url or --static-dir, not both" >&2
  exit 1
fi

if [[ -n "$BASE_URL" ]]; then
  export VQA_BASE_URL="$BASE_URL"
else
  unset VQA_BASE_URL 2>/dev/null || true
fi

if [[ -n "$STATIC_DIR" ]]; then
  export VQA_STATIC_DIR="$STATIC_DIR"
else
  unset VQA_STATIC_DIR 2>/dev/null || true
fi

export VQA_ROUTE="$ROUTE"
export VQA_READY_SELECTOR="$READY_SELECTOR"
export VQA_EXPECTED_TITLE="$EXPECTED_TITLE"
export VQA_SNAPSHOT_NAME="$SNAPSHOT_NAME"
export VQA_PORT="$PORT"

CMD=(npx playwright test)
if [[ "$UPDATE_SNAPSHOTS" -eq 1 ]]; then
  CMD+=(--update-snapshots)
fi
if [[ "$HEADED" -eq 1 ]]; then
  CMD+=(--headed)
fi
if [[ "${#EXTRA_ARGS[@]}" -gt 0 ]]; then
  CMD+=("${EXTRA_ARGS[@]}")
fi

(
  cd "$VQA_DIR"
  "${CMD[@]}"
)
