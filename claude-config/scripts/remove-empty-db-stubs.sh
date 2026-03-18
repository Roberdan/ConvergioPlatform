#!/usr/bin/env bash
# Remove only confirmed 0-byte database stub files (safe)
# Usage: BASE_DIR=/path/to/repo ./remove-empty-db-stubs.sh
set -euo pipefail

BASE_DIR="${BASE_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"

# Absolute path to active dashboard DB that must never be removed
ACTIVE_DASHBOARD_DB="${HOME}/.claude/data/dashboard.db"

# Candidate relative paths (from task description)
candidates=(
  "plan-tasks.db"
  "plan-dashboard.db"
  "plans.db"
  "plan.db"
  "dashboard.db"  # root copy, often a zero-byte stub
  "plan-db.sqlite"
  "plans/plan-db.sqlite"
  "plans/plan.db"
  "plans/plans.db"
  "plans/virtualbpm/plans.db"
  "scripts/plan-db.sqlite"
  "scripts/tasks.db"
  "data/plans.db"
)

removed=()
preserved=()

for rel in "${candidates[@]}"; do
  file="$BASE_DIR/$rel"
  if [[ ! -e "$file" ]]; then
    continue
  fi

  # Resolve symlinks
  realpath="$(cd "$(dirname "$file")" && pwd)/$(basename "$file")"

  # Never touch the active dashboard DB
  if [[ "$realpath" == "$ACTIVE_DASHBOARD_DB" || "$file" == "$ACTIVE_DASHBOARD_DB" ]]; then
    preserved+=("$file (protected)")
    continue
  fi

  # Get size in bytes (portable)
  size=$(wc -c < "$file" 2>/dev/null || echo 0)
  size=$(echo "$size" | tr -d '[:space:]')

  if [[ "$size" == "0" || -z "$size" ]]; then
    rm -f -- "$file"
    echo "REMOVED: $file"
    removed+=("$file")
  else
    echo "PRESERVED (non-zero): $file ($size bytes)"
    preserved+=("$file")
  fi
done

# Summary
if [[ ${#removed[@]} -gt 0 ]]; then
  echo "Removed ${#removed[@]} file(s)"
fi
if [[ ${#preserved[@]} -gt 0 ]]; then
  echo "Preserved ${#preserved[@]} file(s)"
fi

exit 0
