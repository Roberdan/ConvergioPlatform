#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# Tests for 5 tracking hooks — verify structure, no sqlite3, correct shebangs/pipefail
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
    echo "FAIL: $desc — $result"
    FAIL=$((FAIL+1))
  fi
}

HOOKS=(track-tokens.sh track-agent-activity.sh track-precompact.sh track-session-stop.sh session-end-tokens.sh)

for hook in "${HOOKS[@]}"; do
  path="$HOOKS_DIR/$hook"

  # Must exist
  [[ -f "$path" ]] && check "$hook exists" "ok" || { check "$hook exists" "MISSING"; continue; }

  # Must be executable
  [[ -x "$path" ]] && check "$hook is executable" "ok" || check "$hook is executable" "NOT EXECUTABLE"

  # Must start with #!/usr/bin/env bash
  head -1 "$path" | grep -q '#!/usr/bin/env bash' && check "$hook shebang" "ok" || check "$hook shebang" "WRONG SHEBANG"

  # Must have set -euo pipefail
  grep -q 'set -euo pipefail' "$path" && check "$hook pipefail" "ok" || check "$hook pipefail" "MISSING pipefail"

  # Must NOT contain sqlite3 (outside comments)
  ! grep -v '^#' "$path" | grep -q 'sqlite3' && check "$hook no sqlite3" "ok" || check "$hook no sqlite3" "CONTAINS sqlite3"

  # Must call /api/tracking or cvg tracking endpoint
  grep -qE '/api/tracking|cvg tracking' "$path" && check "$hook calls tracking API" "ok" || check "$hook calls tracking API" "NO TRACKING CALL"

  # Must be ≤15 lines
  lines=$(wc -l < "$path")
  [[ $lines -le 15 ]] && check "$hook ≤15 lines ($lines)" "ok" || check "$hook ≤15 lines ($lines)" "TOO LONG: $lines lines"

  # Must have copyright
  grep -q 'Roberto D'\''Angelo' "$path" && check "$hook copyright" "ok" || check "$hook copyright" "MISSING COPYRIGHT"
done

echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]]
