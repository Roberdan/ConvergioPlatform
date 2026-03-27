#!/usr/bin/env bash
# calibrate-models.sh — Weekly model calibration for Convergio kernel.
# Compares local vs cloud model performance, generates routing proposal.
# Runs as nightly job on kernel node (M1 Pro).
set -euo pipefail

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"
SCRIPT_NAME="calibrate-models"

log() { echo "[${SCRIPT_NAME}] $*"; }

# 1. Collect metrics from last 7 days
log "Collecting metrics..."
METRICS=$(curl -sf "${DAEMON_URL}/api/metrics/summary" 2>/dev/null || echo '{}')
PLANS=$(curl -sf "${DAEMON_URL}/api/plan-db/list" 2>/dev/null || echo '{"plans":[]}')

TOTAL_COST=$(echo "$METRICS" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('total_cost_usd',0))" 2>/dev/null || echo "0")
ACTIVE_PLANS=$(echo "$PLANS" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(len([p for p in d.get('plans',[]) if p.get('status')=='doing']))" 2>/dev/null || echo "0")
TOTAL_PLANS=$(echo "$PLANS" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(len(d.get('plans',[])))" 2>/dev/null || echo "0")

log "Total cost: $${TOTAL_COST}, Active plans: ${ACTIVE_PLANS}, Total plans: ${TOTAL_PLANS}"

# 2. Check kernel status
KERNEL=$(curl -sf "${DAEMON_URL}/api/kernel/status" 2>/dev/null || echo '{}')
MODELS_LOADED=$(echo "$KERNEL" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('models_loaded',0))" 2>/dev/null || echo "0")
UPTIME=$(echo "$KERNEL" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('uptime_secs',0))" 2>/dev/null || echo "0")

log "Kernel: ${MODELS_LOADED} models loaded, uptime ${UPTIME}s"

# 3. Check node readiness
READINESS=$(curl -sf "${DAEMON_URL}/api/node/readiness" 2>/dev/null || echo '{"ok":false}')
READY=$(echo "$READINESS" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print('OK' if d.get('ok') else 'ISSUES')" 2>/dev/null || echo "UNKNOWN")

log "Node readiness: ${READY}"

# 4. Generate summary for Telegram
SUMMARY="Calibrazione settimanale Convergio:
- Costo totale: \$${TOTAL_COST}
- Piani attivi: ${ACTIVE_PLANS} / ${TOTAL_PLANS} totali
- Kernel: ${MODELS_LOADED} modello, uptime $(( UPTIME / 3600 ))h
- Nodo: ${READY}
- Prossimo step: MCP integration per risposte intelligenti"

# 5. Send to Telegram if configured
if [[ -n "${CONVERGIO_TELEGRAM_TOKEN:-}" ]] && [[ -n "${CONVERGIO_TELEGRAM_CHAT_ID:-}" ]]; then
  curl -sf "https://api.telegram.org/bot${CONVERGIO_TELEGRAM_TOKEN}/sendMessage" \
    -d "chat_id=${CONVERGIO_TELEGRAM_CHAT_ID}" \
    -d "text=${SUMMARY}" > /dev/null 2>&1 \
    && log "Telegram summary sent" \
    || log "Telegram send failed (non-fatal)"
else
  log "Telegram not configured — skipping notification"
fi

# 6. Submit proposal to evolution engine
curl -sf -X POST "${DAEMON_URL}/api/evolution/proposals" \
  -H 'Content-Type: application/json' \
  -d "{
    \"title\": \"Weekly model calibration $(date +%Y-%m-%d)\",
    \"description\": \"Cost: \$${TOTAL_COST}, Plans: ${ACTIVE_PLANS}, Kernel: ${MODELS_LOADED} models\",
    \"cost_delta\": 0,
    \"risk_score\": 0,
    \"status\": \"Draft\"
  }" > /dev/null 2>&1 || log "Evolution proposal submission failed (non-fatal)"

log "Calibration complete."
