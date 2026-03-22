#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# Tests for 10 lifecycle + utility hooks — structure, no sqlite3, shebangs, <=15 lines
set -euo pipefail

HOOKS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PASS=0
FAIL=0

check() {
  local desc="$1" result="$2"
  if [[ "$result" == "ok" ]]; then
    echo "PASS: $desc"
    PASS=$((PASS+1))
  else
    echo "FAIL: $desc -- $result"
    FAIL=$((FAIL+1))
  fi
}

HOOKS=(
  post-task-enforce.sh
  preserve-context.sh
  inject-agent-context.sh
  notify-app.sh
  worktree-setup.sh
  worktree-teardown.sh
  session-reaper.sh
  session-file-unlock.sh
  model-registry-refresh.sh
)

for hook in "${HOOKS[@]}"; do
  path="$HOOKS_DIR/$hook"

  [[ -f "$path" ]] && check "$hook exists" "ok" || { check "$hook exists" "MISSING"; continue; }
  [[ -x "$path" ]] && check "$hook is executable" "ok" || check "$hook is executable" "NOT EXECUTABLE"

  head -1 "$path" | grep -q '#!/usr/bin/env bash' && check "$hook shebang" "ok" || check "$hook shebang" "WRONG SHEBANG"
  grep -q 'set -.*pipefail' "$path" && check "$hook pipefail" "ok" || check "$hook pipefail" "MISSING pipefail"
  ! grep -q 'sqlite3' "$path" && check "$hook no sqlite3" "ok" || check "$hook no sqlite3" "CONTAINS sqlite3"
  grep -q 'Roberto D'\''Angelo' "$path" && check "$hook copyright" "ok" || check "$hook copyright" "MISSING COPYRIGHT"

  lines=$(wc -l < "$path")
  [[ $lines -le 15 ]] && check "$hook <=15 lines ($lines)" "ok" || check "$hook <=15 lines ($lines)" "TOO LONG: $lines lines"
done

# lib/common.sh: no sqlite3, has copyright, executable
COMMON="$HOOKS_DIR/lib/common.sh"
[[ -f "$COMMON" ]] && check "lib/common.sh exists" "ok" || { check "lib/common.sh exists" "MISSING"; }
[[ -x "$COMMON" ]] && check "lib/common.sh executable" "ok" || check "lib/common.sh executable" "NOT EXECUTABLE"
! grep -q 'sqlite3' "$COMMON" && check "lib/common.sh no sqlite3" "ok" || check "lib/common.sh no sqlite3" "CONTAINS sqlite3"
grep -q 'Roberto D'\''Angelo' "$COMMON" && check "lib/common.sh copyright" "ok" || check "lib/common.sh copyright" "MISSING COPYRIGHT"

echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]]
