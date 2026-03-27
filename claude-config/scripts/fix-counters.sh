#!/bin/bash
set -euo pipefail
# Fix DB counters - ensures plan/wave counters match actual task counts
# Version: 2.0.0 — migrated from sqlite3 to cvg CLI / daemon API
command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"

echo "Fixing counters via daemon API..."

# Get all plan IDs
PLAN_IDS=$(curl -sf "${DAEMON_URL}/api/plan-db/list" 2>/dev/null | jq -r '.[].id' 2>/dev/null) || {
    echo "ERROR: Cannot reach daemon at ${DAEMON_URL}" >&2
    exit 1
}

for plan_id in $PLAN_IDS; do
    echo "Syncing plan $plan_id..."
    # Use the plan sync endpoint to fix counters
    curl -sf -X POST "${DAEMON_URL}/api/plan-db/plan/sync" \
        -H 'Content-Type: application/json' \
        -d "{\"plan_id\":${plan_id}}" >/dev/null 2>&1 || {
        # Fallback: update each wave and plan counter individually via task update triggers
        # TODO: needs daemon endpoint for counter sync
        echo "  WARNING: sync API not available for plan $plan_id, attempting wave-level fix" >&2

        # Read plan JSON and check for mismatches
        PJ=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${plan_id}" 2>/dev/null) || continue

        # Check wave counter mismatches
        WAVE_ISSUES=$(echo "$PJ" | jq -r '
            [.waves[]? | {
                wave_id: .wave_id,
                db_id: (.id // .db_id),
                recorded_done: (.tasks_done // 0),
                actual_done: ([.tasks[]? | select(.status == "done")] | length),
                recorded_total: (.tasks_total // 0),
                actual_total: ([.tasks[]?] | length)
            } | select(.recorded_done != .actual_done or .recorded_total != .actual_total)]
            | if length > 0 then .[] | "\(.wave_id) (id=\(.db_id)): recorded=\(.recorded_done)/\(.recorded_total) actual=\(.actual_done)/\(.actual_total)" else empty end
        ' 2>/dev/null || echo "")

        if [[ -n "$WAVE_ISSUES" ]]; then
            echo "  Counter mismatches found:"
            echo "$WAVE_ISSUES" | while IFS= read -r line; do
                echo "    $line"
            done
            # Try to fix via wave update API
            echo "$PJ" | jq -r '.waves[]? | .id // .db_id' 2>/dev/null | while read -r wid; do
                curl -sf -X POST "${DAEMON_URL}/api/plan-db/wave/update" \
                    -H 'Content-Type: application/json' \
                    -d "{\"wave_db_id\":${wid},\"sync_counters\":true}" >/dev/null 2>&1 || true
            done
        fi
    }
done

echo ""
echo "Verification:"
MISMATCH_COUNT=0
for plan_id in $PLAN_IDS; do
    PJ=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${plan_id}" 2>/dev/null) || continue
    PLAN_MISMATCHES=$(echo "$PJ" | jq '
        (.tasks_done // 0) as $pd |
        ([.waves[]? | .tasks_done // 0] | add // 0) as $ad |
        (.tasks_total // 0) as $pt |
        ([.waves[]? | .tasks_total // 0] | add // 0) as $at |
        if $pd != $ad or $pt != $at then 1 else 0 end
    ' 2>/dev/null || echo "0")
    WAVE_MISMATCHES=$(echo "$PJ" | jq '
        [.waves[]? | select(
            (.tasks_done // 0) != ([.tasks[]? | select(.status == "done")] | length) or
            (.tasks_total // 0) != ([.tasks[]?] | length)
        )] | length
    ' 2>/dev/null || echo "0")
    MISMATCH_COUNT=$((MISMATCH_COUNT + PLAN_MISMATCHES + WAVE_MISMATCHES))
done

echo "Mismatched counters: $MISMATCH_COUNT"
if [[ "$MISMATCH_COUNT" -gt 0 ]]; then
    echo "WARNING: Some counters still mismatched after sync"
fi
echo "Done."
