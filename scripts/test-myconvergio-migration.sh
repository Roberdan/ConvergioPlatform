#!/usr/bin/env bash
set -euo pipefail

# Verify migration from MyConvergio to Convergio is complete:
# 1. No MyConvergio references remain in source files
# 2. All shell scripts pass bash -n syntax check

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
ERRORS=0

cleanup() {
  if [[ "${ERRORS}" -gt 0 ]]; then
    echo "FAIL: ${ERRORS} error(s) found"
    exit 1
  fi
  echo "PASS: Migration verification complete"
}
trap cleanup EXIT

echo "=== MyConvergio Reference Check ==="
echo "Scanning for MyConvergio references in source files..."

# Exclude binary files, node_modules, .git, and this script itself
local_matches=$(grep -ri "myconvergio" \
  --include="*.rs" --include="*.js" --include="*.ts" --include="*.sh" \
  --include="*.html" --include="*.css" --include="*.json" --include="*.yaml" \
  --include="*.yml" --include="*.toml" --include="*.md" \
  -l "${REPO_ROOT}" 2>/dev/null \
  | grep -v "node_modules" \
  | grep -v ".git/" \
  | grep -v "CHANGELOG.md" \
  | grep -v "TROUBLESHOOTING.md" \
  | grep -v "test-myconvergio-migration.sh" \
  || true)

if [[ -n "${local_matches}" ]]; then
  echo "ERROR: Found MyConvergio references in:"
  echo "${local_matches}"
  ERRORS=$((ERRORS + ${#local_matches[@]}))
else
  echo "OK: No MyConvergio references found in source files"
fi

echo ""
echo "=== Shell Script Syntax Check ==="
echo "Running bash -n on all .sh files..."

while IFS= read -r script; do
  local_rel="${script#"${REPO_ROOT}/"}"
  if bash -n "${script}" 2>/dev/null; then
    echo "  OK: ${local_rel}"
  else
    echo "  FAIL: ${local_rel}"
    ERRORS=$((ERRORS + 1))
  fi
done < <(find "${REPO_ROOT}/scripts" -name "*.sh" -type f | sort)

# Also check daemon/start.sh and dashboard/start.sh if they exist
for extra in "${REPO_ROOT}/daemon/start.sh" "${REPO_ROOT}/dashboard/start.sh"; do
  if [[ -f "${extra}" ]]; then
    local_rel="${extra#"${REPO_ROOT}/"}"
    if bash -n "${extra}" 2>/dev/null; then
      echo "  OK: ${local_rel}"
    else
      echo "  FAIL: ${local_rel}"
      ERRORS=$((ERRORS + 1))
    fi
  fi
done

echo ""
echo "=== Summary ==="
echo "Errors: ${ERRORS}"
