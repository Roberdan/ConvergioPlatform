#!/bin/bash
set -euo pipefail
# Thor Quick Validation - Runs all validations in one command
# Reduces tokens by running via bash instead of inline agent work
# Usage: thor-validate.sh <plan_id> [--full]

# Version: 3.0.0 - Uses daemon API instead of direct sqlite3 calls
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLAN_ID="${1:?Usage: thor-validate.sh <plan_id> [--full]}"
FULL_CHECK="${2:-}"
DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "=========================================="
echo "THOR VALIDATION - Plan $PLAN_ID"
echo "=========================================="
echo ""

ERRORS=0

# 0. Validate submitted tasks → done (per-task Thor)
PLAN_JSON=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${PLAN_ID}" || echo '{}')
SUBMITTED_TASKS=$(echo "$PLAN_JSON" | jq -r '.tasks[]? | select(.status=="submitted") | "\(.id)|\(.task_id)"')
if [[ -n "$SUBMITTED_TASKS" ]]; then
	SUBMITTED_COUNT=$(echo "$SUBMITTED_TASKS" | wc -l | tr -d ' ')
	echo "[0/5] Per-task validation ($SUBMITTED_COUNT submitted tasks)..."
	while IFS='|' read -r db_id task_id; do
		if "$SCRIPT_DIR/plan-db.sh" validate-task "$db_id" "$PLAN_ID" thor 2>/dev/null; then
			echo -e "  ${GREEN}$task_id: submitted → done${NC}"
		else
			echo -e "  ${RED}$task_id: validation FAILED${NC}"
			ERRORS=$((ERRORS + 1))
		fi
	done <<<"$SUBMITTED_TASKS"
else
	echo "[0/5] Per-task validation... no submitted tasks"
fi

# 1. Database validation (bulk — validates already-done tasks)
echo "[1/5] Database integrity..."
if "$SCRIPT_DIR/plan-db.sh" validate "$PLAN_ID" >/tmp/thor_db_$$.log 2>&1; then
	echo -e "${GREEN}  PASS${NC}"
else
	echo -e "${RED}  FAIL - see /tmp/thor_db_$$.log${NC}"
	ERRORS=$((ERRORS + 1))
fi

# 2. F-xx validation
echo "[2/5] F-xx requirements..."
if "$SCRIPT_DIR/plan-db.sh" validate-fxx "$PLAN_ID" >/tmp/thor_fxx_$$.log 2>&1; then
	echo -e "${GREEN}  PASS${NC}"
else
	echo -e "${RED}  FAIL - see /tmp/thor_fxx_$$.log${NC}"
	ERRORS=$((ERRORS + 1))
fi

# 3. Build check (if --full)
if [[ "$FULL_CHECK" == "--full" ]]; then
	echo "[3/5] Build check..."
	if npm run lint 2>&1 | tail -10 >/tmp/thor_lint_$$.log &&
		npm run typecheck 2>&1 | tail -10 >/tmp/thor_type_$$.log &&
		npm run build 2>&1 | tail -10 >/tmp/thor_build_$$.log; then
		echo -e "${GREEN}  PASS${NC}"
	else
		echo -e "${RED}  FAIL - check logs in /tmp/thor_*_$$.log${NC}"
		ERRORS=$((ERRORS + 1))
	fi
else
	echo "[3/5] Build check... SKIPPED (use --full)"
fi

# 4. File size check (project source files, not ~/.claude scripts)
PROJECT_PATH="${PROJECT_PATH:-.}"
PROJECT_PATH=$(cd "$PROJECT_PATH" 2>/dev/null && pwd) || {
	echo -e "${RED}  FAIL - invalid project path${NC}"
	ERRORS=$((ERRORS + 1))
	PROJECT_PATH=""
}

echo "[4/5] File sizes (<250 lines)..."
LARGE_FILES=""
if [[ -n "$PROJECT_PATH" ]] && [[ -d "$PROJECT_PATH/src" ]]; then
	LARGE_FILES=$(find "$PROJECT_PATH/src" -name "*.ts" -o -name "*.tsx" 2>/dev/null |
		grep -v node_modules | grep -v '.test.' |
		while read -r f; do
			lines=$(/usr/bin/wc -l <"$f" 2>/dev/null | tr -d ' ')
			[[ $lines -gt 250 ]] && echo "$f: $lines"
		done)
fi

if [[ -z "$LARGE_FILES" ]]; then
	echo -e "${GREEN}  PASS${NC}"
else
	echo -e "${YELLOW}  WARNING - Large files:${NC}"
	echo "$LARGE_FILES" | head -5
	ERRORS=$((ERRORS + 1))
fi

# 5. Remaining submitted check (should be 0 after step 0)
echo "[5/5] No tasks stuck in submitted..."
# Re-fetch plan data to check for remaining submitted tasks after validation
PLAN_JSON_REFRESH=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${PLAN_ID}" || echo '{}')
STILL_SUBMITTED=$(echo "$PLAN_JSON_REFRESH" | jq '[.tasks[]? | select(.status=="submitted")] | length')
if [[ "${STILL_SUBMITTED:-0}" -eq 0 ]]; then
	echo -e "${GREEN}  PASS${NC}"
else
	echo -e "${RED}  FAIL - $STILL_SUBMITTED tasks still submitted (Thor validation failed)${NC}"
	ERRORS=$((ERRORS + 1))
fi

echo ""
echo "=========================================="
if [[ $ERRORS -eq 0 ]]; then
	echo -e "${GREEN}THOR VALIDATION: PASSED${NC}"
	exit 0
else
	echo -e "${RED}THOR VALIDATION: FAILED ($ERRORS issues)${NC}"
	exit 1
fi
