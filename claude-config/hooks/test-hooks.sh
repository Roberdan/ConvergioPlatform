#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo
# Master smoke test — verifies all 21 hooks in claude-config/hooks/.
# Runs structural checks then delegates to existing sub-test scripts.
set -euo pipefail

HOOKS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PASS=0
FAIL=0

check() {
  local desc="$1" result="$2"
  if [[ "$result" == "ok" ]]; then
    echo "PASS: $desc"
    PASS=$((PASS + 1))
  else
    echo "FAIL: $desc -- $result"
    FAIL=$((FAIL + 1))
  fi
}

# All hooks (excluding test scripts and the master itself)
ENFORCEMENT_HOOKS=(
  enforce-planner-workflow.sh
  workflow-enforcer.sh
  env-vault-guard.sh
  worktree-guard.sh
  version-check.sh
)

TRACKING_HOOKS=(
  track-tokens.sh
  track-agent-activity.sh
  track-precompact.sh
  track-session-stop.sh
  session-end-tokens.sh
)

LIFECYCLE_HOOKS=(
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

ALL_HOOKS=("${ENFORCEMENT_HOOKS[@]}" "${TRACKING_HOOKS[@]}" "${LIFECYCLE_HOOKS[@]}")

echo "=== Hook Structural Checks (${#ALL_HOOKS[@]} hooks) ==="
for hook in "${ALL_HOOKS[@]}"; do
  path="$HOOKS_DIR/$hook"
  [[ -f "$path" ]] && check "$hook exists" "ok" || { check "$hook exists" "MISSING"; continue; }
  [[ -x "$path" ]] && check "$hook executable" "ok" || check "$hook executable" "NOT EXECUTABLE"
  head -1 "$path" | grep -q '#!/usr/bin/env bash' && check "$hook shebang" "ok" || check "$hook shebang" "WRONG SHEBANG"
  grep -q 'set -.*pipefail' "$path" && check "$hook pipefail" "ok" || check "$hook pipefail" "MISSING pipefail"
  ! grep -v '^#' "$path" | grep -q 'sqlite3' && check "$hook no sqlite3" "ok" || check "$hook no sqlite3" "CONTAINS sqlite3"
  grep -q "Roberto D'Angelo" "$path" && check "$hook copyright" "ok" || check "$hook copyright" "MISSING COPYRIGHT"
done

echo ""
echo "=== Enforcement Hook Behavior Checks ==="

# enforce-planner-workflow: blocks EnterPlanMode
RESULT=$(echo '{"tool_name":"EnterPlanMode"}' | bash "$HOOKS_DIR/enforce-planner-workflow.sh" 2>/dev/null)
echo "$RESULT" | grep -q '"block"' && check "enforce-planner-workflow blocks EnterPlanMode" "ok" || check "enforce-planner-workflow blocks EnterPlanMode" "NOT BLOCKED"

# enforce-planner-workflow: blocks plan-db.sh create
RESULT=$(printf '{"tool_name":"Bash","tool_input":{"command":"plan-db.sh create myproject Test"}}' | bash "$HOOKS_DIR/enforce-planner-workflow.sh" 2>/dev/null)
echo "$RESULT" | grep -q '"block"' && check "enforce-planner-workflow blocks plan-db.sh create" "ok" || check "enforce-planner-workflow blocks plan-db.sh create" "NOT BLOCKED"

# enforce-planner-workflow: blocks plan-db.sh update-task done
RESULT=$(printf '{"tool_name":"Bash","tool_input":{"command":"plan-db.sh update-task 123 done summary"}}' | bash "$HOOKS_DIR/enforce-planner-workflow.sh" 2>/dev/null)
echo "$RESULT" | grep -q '"block"' && check "enforce-planner-workflow blocks update-task done" "ok" || check "enforce-planner-workflow blocks update-task done" "NOT BLOCKED"

# enforce-planner-workflow: allows plan-db-safe.sh
RESULT=$(printf '{"tool_name":"Bash","tool_input":{"command":"plan-db-safe.sh update-task 123 done summary"}}' | bash "$HOOKS_DIR/enforce-planner-workflow.sh" 2>/dev/null)
[[ -z "$RESULT" ]] && check "enforce-planner-workflow allows plan-db-safe.sh" "ok" || check "enforce-planner-workflow allows plan-db-safe.sh" "SHOULD NOT BLOCK: $RESULT"

# workflow-enforcer: blocks EnterPlanMode
RESULT=$(echo '{"tool_name":"EnterPlanMode"}' | bash "$HOOKS_DIR/workflow-enforcer.sh" 2>/dev/null)
echo "$RESULT" | grep -q '"block"' && check "workflow-enforcer blocks EnterPlanMode" "ok" || check "workflow-enforcer blocks EnterPlanMode" "NOT BLOCKED"

# worktree-guard: exits 1 with no args (set +e to allow non-zero exit)
set +e
bash "$HOOKS_DIR/worktree-guard.sh" 2>/dev/null; EC=$?
set -e
[[ $EC -eq 1 ]] && check "worktree-guard exits 1 with no args" "ok" || check "worktree-guard exits 1 with no args" "EXIT $EC"

echo ""
echo "=== lib/common.sh Checks ==="
COMMON="$HOOKS_DIR/lib/common.sh"
[[ -f "$COMMON" ]] && check "lib/common.sh exists" "ok" || { check "lib/common.sh exists" "MISSING"; }
[[ -x "$COMMON" ]] && check "lib/common.sh executable" "ok" || check "lib/common.sh executable" "NOT EXECUTABLE"
! grep -q 'sqlite3' "$COMMON" && check "lib/common.sh no sqlite3" "ok" || check "lib/common.sh no sqlite3" "CONTAINS sqlite3"
grep -q "Roberto D'Angelo" "$COMMON" && check "lib/common.sh copyright" "ok" || check "lib/common.sh copyright" "MISSING COPYRIGHT"

echo ""
echo "=== Tracking Hook Sub-Tests ==="
set +e
bash "$HOOKS_DIR/test_tracking_hooks.sh"; EC=$?
set -e
[[ $EC -eq 0 ]] && check "test_tracking_hooks.sh all pass" "ok" || check "test_tracking_hooks.sh all pass" "FAILURES (see above)"

echo ""
echo "=== Lifecycle Hook Sub-Tests ==="
set +e
bash "$HOOKS_DIR/test_lifecycle_hooks.sh"; EC=$?
set -e
[[ $EC -eq 0 ]] && check "test_lifecycle_hooks.sh all pass" "ok" || check "test_lifecycle_hooks.sh all pass" "FAILURES (see above)"

echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]]
