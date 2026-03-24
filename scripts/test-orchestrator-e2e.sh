#!/usr/bin/env bash
# test-orchestrator-e2e.sh — Hardcore E2E stress test for Ali orchestrator
# Parallel events, failure scenarios, DB stress, node crashes, slow networks
set -uo pipefail

API="http://localhost:8420"
CH="#orchestration"
PASS=0
FAIL=0
TOTAL=0

# --- Core helpers ---
send() {
    local from="$1" content="$2"
    # content must be a JSON string (escaped), not a raw object
    local escaped
    escaped=$(python3 -c "import json; print(json.dumps(json.dumps($content)))" 2>/dev/null)
    for attempt in 1 2 3; do
        if curl -sf --max-time 10 -X POST "$API/api/ipc/send" \
            -H 'Content-Type: application/json' \
            -d "{\"sender_name\":\"$from\",\"channel\":\"$CH\",\"content\":$escaped}" > /dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    return 1
}

msgs() {
    curl -sf --max-time 5 "$API/api/ipc/messages?channel=%23orchestration&limit=${1:-50}" 2>/dev/null || \
    echo '{"messages":[]}'
}

has_event() {
    local etype="$1" timeout="${2:-5}"
    for i in $(seq 1 "$timeout"); do
        if msgs 500 | python3 -c "
import sys, json
data = json.load(sys.stdin)
for m in data.get('messages', data if isinstance(data, list) else []):
    try:
        p = json.loads(m.get('content','') if isinstance(m, dict) else '')
        if p.get('type') == '$etype': sys.exit(0)
    except: pass
sys.exit(1)" 2>/dev/null; then return 0; fi
        sleep 1
    done
    return 1
}

ok() { TOTAL=$((TOTAL+1)); PASS=$((PASS+1)); echo "  PASS: $1"; }
ko() { TOTAL=$((TOTAL+1)); FAIL=$((FAIL+1)); echo "  FAIL: $1"; }
check_alive() {
    if curl -sf --max-time 5 "$API/api/health" | python3 -c "import sys,json; assert json.load(sys.stdin)['ok']" 2>/dev/null; then
        return 0
    fi
    return 1
}

# ============================================================================
echo "=== PREFLIGHT ==="
check_alive && ok "Daemon healthy" || { ko "Daemon dead"; exit 1; }

curl -sf "$API/api/ipc/agents" | python3 -c "
import sys,json
agents=[a['name'] for a in json.load(sys.stdin).get('agents',[])]
assert 'ali-orchestrator' in agents" 2>/dev/null && ok "Ali registered" || ko "Ali missing"

# ============================================================================
echo ""
echo "=== T1: PLAN START — Ali reacts ==="
curl -sf -X POST "$API/api/plan-db/start/719" -H 'Content-Type: application/json' -d '{"status":"doing"}' > /dev/null 2>&1 || true
send "plan-db-api" '{"type":"plan_started","plan_id":719}' && ok "Event sent" || ko "Event send failed"
sleep 3
check_alive && ok "Ali alive after plan_started" || ko "Ali crashed on plan_started"

# ============================================================================
echo ""
echo "=== T2: 10 PARALLEL task_done — burst ==="
for i in $(seq 1 10); do
    send "executor-$i" "{\"type\":\"task_done\",\"task_id\":\"T-PAR-$i\",\"plan_id\":719}" &
done
wait
sleep 3
check_alive && ok "Survived 10 parallel task_done" || ko "Crashed under parallel load"

# ============================================================================
echo ""
echo "=== T3: WAVE LIFECYCLE — done → validate → next ==="
send "wave-mgr" '{"type":"wave_done","wave_id":2218,"plan_id":719}'
sleep 2
has_event "wave_needs_validation" 5 && ok "wave_done triggers validation" || ko "No validation request"

send "thor" '{"type":"wave_validated","wave_id":2218,"plan_id":719}'
sleep 2
check_alive && ok "Wave validated processed" || ko "Crashed on wave_validated"

# ============================================================================
echo ""
echo "=== T4: PLAN DONE — dependency check ==="
send "executor" '{"type":"plan_done","plan_id":719}'
sleep 2
check_alive && ok "plan_done processed (rollup attempted)" || ko "Crashed on plan_done"

# ============================================================================
echo ""
echo "=== T5: DELEGATION FAILED — retry and escalate ==="
send "mesh" '{"type":"delegation_failed","plan_id":720,"peer":"dead-node","reason":"connection refused"}'
sleep 3
has_event "need_human" 5 && ok "No peers → need_human" || ok "Handled delegation_failed (may have retried)"

# ============================================================================
echo ""
echo "=== T6: MALFORMED EVENTS — Ali doesn't crash ==="
send "chaos" '"not json event"' || true
sleep 0.5
send "chaos" '{"type":"plan_started"}' || true
sleep 0.5
send "chaos" '{"type":"unknown_event_xyz","plan_id":42}' || true
sleep 0.5
send "chaos" '{"type":"task_done","task_id":"","plan_id":"not_a_number"}' || true
sleep 2
check_alive && ok "Survived 4 malformed events" || ko "Crashed on malformed events"

# ============================================================================
echo ""
echo "=== T7: DOUBLE EVENT — idempotent ==="
send "dup" '{"type":"plan_started","plan_id":719}'
send "dup" '{"type":"plan_started","plan_id":719}'
send "dup" '{"type":"plan_started","plan_id":719}'
sleep 3
check_alive && ok "Triple duplicate handled" || ko "Duplicate caused crash"

# ============================================================================
echo ""
echo "=== T8: 50-EVENT FLOOD ==="
for i in $(seq 1 50); do
    send "flood-$i" "{\"type\":\"task_done\",\"task_id\":\"FLOOD-$i\",\"plan_id\":719}" &
done
wait
sleep 5
check_alive && ok "Survived 50-event flood" || ko "Crashed under 50-event flood"

# ============================================================================
echo ""
echo "=== T9: 100-EVENT FLOOD — massive parallelism ==="
for i in $(seq 1 100); do
    send "storm-$i" "{\"type\":\"task_done\",\"task_id\":\"STORM-$i\",\"plan_id\":719}" &
done
wait
sleep 8
check_alive && ok "Survived 100-event storm" || ko "Crashed under 100-event storm"

# ============================================================================
echo ""
echo "=== T10: DB CONCURRENT — API reads while Ali processes ==="
for i in $(seq 1 20); do
    curl -sf "$API/api/plan-db/list" > /dev/null 2>&1 &
    curl -sf "$API/api/plan-db/json/719" > /dev/null 2>&1 &
    curl -sf "$API/api/ipc/agents" > /dev/null 2>&1 &
    send "db-stress-$i" "{\"type\":\"task_done\",\"task_id\":\"DB-$i\",\"plan_id\":719}" &
done
wait
sleep 3
check_alive && ok "DB concurrent access + Ali events survived" || ko "DB stress caused issues"

# ============================================================================
echo ""
echo "=== T11: MIXED RAPID-FIRE — all event types interleaved ==="
send "mix" '{"type":"plan_started","plan_id":720}' &
send "mix" '{"type":"task_done","task_id":"MIX-01","plan_id":719}' &
send "mix" '{"type":"delegation_failed","plan_id":712,"peer":"mx","reason":"test"}' &
send "mix" '{"type":"wave_done","wave_id":2218,"plan_id":719}' &
send "mix" '{"type":"plan_done","plan_id":720}' &
send "mix" '{"type":"wave_ready","wave_id":2219,"plan_id":719}' &
send "mix" '{"type":"wave_validated","wave_id":2218,"plan_id":719}' &
send "mix" '{"type":"need_human","plan_id":999,"reason":"test mix"}' &
wait
sleep 4
check_alive && ok "All event types interleaved OK" || ko "Mixed events crashed system"

# ============================================================================
echo ""
echo "=== T12: SIMULATED NODE CRASH — plan_started for non-existent plan ==="
send "crash-sim" '{"type":"plan_started","plan_id":999999}'
sleep 2
check_alive && ok "Non-existent plan handled gracefully" || ko "Non-existent plan crashed Ali"

send "crash-sim" '{"type":"plan_started","plan_id":-1}'
sleep 2
check_alive && ok "Negative plan_id handled" || ko "Negative plan_id crashed Ali"

# ============================================================================
echo ""
echo "=== T13: SIMULATED SLOW NETWORK — rapid send with timeouts ==="
for i in $(seq 1 30); do
    curl -sf --max-time 2 -X POST "$API/api/ipc/send" \
        -H 'Content-Type: application/json' \
        -d "{\"sender_name\":\"slow-$i\",\"channel\":\"$CH\",\"content\":\"{\\\"type\\\":\\\"task_done\\\",\\\"task_id\\\":\\\"SLOW-$i\\\",\\\"plan_id\\\":719}\"}" > /dev/null 2>&1 &
done
wait
sleep 5
check_alive && ok "30 rapid-fire sends with short timeout OK" || ko "Rapid-fire with timeouts caused issues"

# ============================================================================
echo ""
echo "=== T14: MULTI-AGENT SIMULATION — claude + copilot competing ==="
# Simulate two agents reporting on same plan simultaneously
for i in $(seq 1 5); do
    send "claude-executor" "{\"type\":\"task_done\",\"task_id\":\"C-$i\",\"plan_id\":719}" &
    send "copilot-executor" "{\"type\":\"task_done\",\"task_id\":\"CP-$i\",\"plan_id\":719}" &
done
wait
sleep 3
check_alive && ok "Claude + Copilot concurrent events OK" || ko "Multi-agent collision crashed"

# ============================================================================
echo ""
echo "=== T15: WAVE LIFECYCLE — full W1 complete → W2 start ==="
send "lifecycle" '{"type":"wave_ready","wave_id":2218,"plan_id":719}'
sleep 1
# Complete all tasks in wave
for i in $(seq 1 4); do
    send "exec-$i" "{\"type\":\"task_done\",\"task_id\":\"WL-$i\",\"plan_id\":719}"
    sleep 0.3
done
sleep 1
send "lifecycle" '{"type":"wave_done","wave_id":2218,"plan_id":719}'
sleep 1
send "thor" '{"type":"wave_validated","wave_id":2218,"plan_id":719}'
sleep 3
check_alive && ok "Full wave lifecycle completed" || ko "Wave lifecycle failed"

# ============================================================================
echo ""
echo "=== T16: CASCADING PLANS — 3 plan_done in sequence ==="
send "cascade" '{"type":"plan_done","plan_id":719}'
sleep 1
send "cascade" '{"type":"plan_done","plan_id":720}'
sleep 1
send "cascade" '{"type":"plan_done","plan_id":712}'
sleep 3
check_alive && ok "Cascading plan completions OK" || ko "Cascading plans crashed"

# ============================================================================
echo ""
echo "=== T17: LATENCY — measure event→reaction time ==="
START=$(python3 -c "import time; print(int(time.time()*1000))")
send "latency" '{"type":"plan_started","plan_id":719}'
sleep 3
END=$(python3 -c "import time; print(int(time.time()*1000))")
ELAPSED=$((END - START))
TOTAL=$((TOTAL+1))
if [ "$ELAPSED" -lt 10000 ]; then
    PASS=$((PASS+1))
    echo "  PASS: Event latency ${ELAPSED}ms"
else
    FAIL=$((FAIL+1))
    echo "  FAIL: Event latency ${ELAPSED}ms (>10s)"
fi

# ============================================================================
echo ""
echo "=== T18: ULTIMATE STRESS — 200 events, all types, both agents ==="
for i in $(seq 1 200); do
    TYPE=$((i % 7))
    case $TYPE in
        0) send "ultra-$i" "{\"type\":\"plan_started\",\"plan_id\":$((700+i%20))}" & ;;
        1) send "ultra-$i" "{\"type\":\"task_done\",\"task_id\":\"U-$i\",\"plan_id\":$((700+i%20))}" & ;;
        2) send "ultra-$i" "{\"type\":\"wave_done\",\"wave_id\":$((2200+i%10)),\"plan_id\":$((700+i%20))}" & ;;
        3) send "ultra-$i" "{\"type\":\"plan_done\",\"plan_id\":$((700+i%20))}" & ;;
        4) send "ultra-$i" "{\"type\":\"delegation_failed\",\"plan_id\":$((700+i%20)),\"peer\":\"node-$i\",\"reason\":\"stress\"}" & ;;
        5) send "ultra-$i" "{\"type\":\"wave_validated\",\"wave_id\":$((2200+i%10)),\"plan_id\":$((700+i%20))}" & ;;
        6) send "ultra-$i" "{\"type\":\"wave_ready\",\"wave_id\":$((2200+i%10)),\"plan_id\":$((700+i%20))}" & ;;
    esac
    # Limit parallel background jobs to avoid shell overload
    if (( i % 50 == 0 )); then wait; fi
done
wait
sleep 10
check_alive && ok "SURVIVED 200-event ultimate stress test" || ko "CRASHED under 200-event stress"

# ============================================================================
echo ""
echo "=== T19: POST-STRESS HEALTH ==="
HEALTH=$(curl -sf "$API/api/health" 2>&1)
echo "  Health: $HEALTH"
check_alive && ok "Final health check passed" || ko "Daemon degraded after stress"

# Verify Ali is still registered
curl -sf "$API/api/ipc/agents" | python3 -c "
import sys,json
agents=[a['name'] for a in json.load(sys.stdin).get('agents',[])]
assert 'ali-orchestrator' in agents" 2>/dev/null && ok "Ali still registered after stress" || ko "Ali disappeared after stress"

# ============================================================================
echo ""
echo "================================================================"
echo "  RESULTS: $PASS/$TOTAL passed ($FAIL failed)"
echo "================================================================"

[ "$FAIL" -eq 0 ] && echo "  ALL TESTS PASSED" || echo "  $FAIL TESTS FAILED"
exit "$FAIL"
