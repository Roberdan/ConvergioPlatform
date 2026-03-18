#!/usr/bin/env bash
# plan-db-api.sh — Thin curl wrapper around claude-core Plan DB API
# Replaces direct sqlite3 calls with HTTP API calls to the daemon.
# Same CLI interface as plan-db.sh for drop-in replacement.
set -euo pipefail

DAEMON_URL="${CLAUDE_DAEMON_URL:-http://localhost:8420}"
AUTH_TOKEN="${CLAUDE_API_TOKEN:-}"

api() {
  local method="$1" path="$2"; shift 2
  local args=(-sf -H "Content-Type: application/json")
  [[ -n "$AUTH_TOKEN" ]] && args+=(-H "Authorization: Bearer $AUTH_TOKEN")
  if [[ "$method" == "GET" ]]; then
    curl "${args[@]}" "${DAEMON_URL}${path}" "$@"
  else
    curl "${args[@]}" -X "$method" "${DAEMON_URL}${path}" "$@"
  fi
}

json_field() { python3 -c "import sys,json;print(json.load(sys.stdin).get('$1',''))"; }

case "${1:-}" in
  list)
    shift; api GET "/api/plan-db/list" | python3 -m json.tool
    ;;
  create)
    shift; local proj="${1:-}"; local name="${2:-}"
    [[ -z "$proj" || -z "$name" ]] && { echo "Usage: plan-db.sh create <project_id> <name>"; exit 1; }
    api POST "/api/plan-db/create" -d "{\"project_id\":\"$proj\",\"name\":\"$name\"}"
    ;;
  start)
    shift; local plan_id="${1:-}"
    [[ -z "$plan_id" ]] && { echo "Usage: plan-db.sh start <plan_id>"; exit 1; }
    api POST "/api/plan-db/start/$plan_id"
    ;;
  complete)
    shift; local plan_id="${1:-}"
    [[ -z "$plan_id" ]] && { echo "Usage: plan-db.sh complete <plan_id>"; exit 1; }
    api POST "/api/plan-db/complete/$plan_id"
    ;;
  cancel)
    shift; local plan_id="${1:-}"; local reason="${2:-cancelled}"
    [[ -z "$plan_id" ]] && { echo "Usage: plan-db.sh cancel <plan_id> [reason]"; exit 1; }
    api POST "/api/plan-db/cancel/$plan_id" -d "{\"reason\":\"$reason\"}"
    ;;
  approve)
    shift; local plan_id="${1:-}"
    [[ -z "$plan_id" ]] && { echo "Usage: plan-db.sh approve <plan_id>"; exit 1; }
    api POST "/api/plan-db/approve/$plan_id"
    ;;
  update-task)
    shift; local task_id="${1:-}"; local status="${2:-}"; shift 2 || true
    local notes="${*:-}"
    [[ -z "$task_id" || -z "$status" ]] && { echo "Usage: plan-db.sh update-task <id> <status> [notes]"; exit 1; }
    local payload="{\"task_id\":$task_id,\"status\":\"$status\""
    [[ -n "$notes" ]] && payload+=",\"notes\":\"$notes\""
    payload+="}"
    api POST "/api/plan-db/task/update" -d "$payload"
    ;;
  json)
    shift; local plan_id="${1:-}"
    [[ -z "$plan_id" ]] && { echo "Usage: plan-db.sh json <plan_id>"; exit 1; }
    api GET "/api/plan-db/json/$plan_id" | python3 -m json.tool
    ;;
  get-context)
    shift; local plan_id="${1:-}"
    [[ -z "$plan_id" ]] && { echo "Usage: plan-db.sh get-context <plan_id>"; exit 1; }
    api GET "/api/plan-db/context/$plan_id" | python3 -m json.tool
    ;;
  execution-tree)
    shift; local plan_id="${1:-}"
    [[ -z "$plan_id" ]] && { echo "Usage: plan-db.sh execution-tree <plan_id>"; exit 1; }
    api GET "/api/plan-db/execution-tree/$plan_id" | python3 -m json.tool
    ;;
  drift-check)
    shift; local plan_id="${1:-}"
    [[ -z "$plan_id" ]] && { echo "Usage: plan-db.sh drift-check <plan_id>"; exit 1; }
    api GET "/api/plan-db/drift-check/$plan_id" | python3 -m json.tool
    ;;
  validate-task)
    shift; local task_id="${1:-}"; local plan_id="${2:-}"
    [[ -z "$task_id" || -z "$plan_id" ]] && { echo "Usage: plan-db.sh validate-task <task_id> <plan_id>"; exit 1; }
    api GET "/api/plan-db/validate-task/$task_id/$plan_id" | python3 -m json.tool
    ;;
  import)
    shift; local plan_id="${1:-}"; local spec_file="${2:-}"
    [[ -z "$plan_id" || -z "$spec_file" ]] && { echo "Usage: plan-db.sh import <plan_id> <spec.json|spec.yaml>"; exit 1; }
    local content; content=$(cat "$spec_file")
    if [[ "$spec_file" == *.yaml || "$spec_file" == *.yml ]]; then
      api POST "/api/plan-db/import" -d "{\"plan_id\":$plan_id,\"spec\":$(python3 -c "import sys,json;print(json.dumps(sys.stdin.read()))" <<< "$content")}"
    else
      local merged; merged=$(python3 -c "import sys,json;d=json.load(sys.stdin);d['plan_id']=$plan_id;print(json.dumps(d))" <<< "$content")
      api POST "/api/plan-db/import" -d "$merged"
    fi
    ;;
  wave-update|update-wave)
    shift; local wave_id="${1:-}"; local status="${2:-}"
    [[ -z "$wave_id" || -z "$status" ]] && { echo "Usage: plan-db.sh update-wave <wave_id> <status>"; exit 1; }
    api POST "/api/plan-db/wave/update" -d "{\"wave_id\":$wave_id,\"status\":\"$status\"}"
    ;;
  kb-search)
    shift; local query="${1:-}"
    [[ -z "$query" ]] && { echo "Usage: plan-db.sh kb-search <query>"; exit 1; }
    api GET "/api/plan-db/kb-search?q=$(python3 -c "import urllib.parse;print(urllib.parse.quote('$query'))")"
    ;;
  agent-start)
    shift; local agent_id="${1:-}"; local agent_type="${2:-}"; local desc="${3:-}"
    api POST "/api/plan-db/agent/start" -d "{\"agent_id\":\"$agent_id\",\"agent_type\":\"$agent_type\",\"description\":\"$desc\"}"
    ;;
  agent-complete)
    shift; local agent_id="${1:-}"
    api POST "/api/plan-db/agent/complete" -d "{\"agent_id\":\"$agent_id\"}"
    ;;
  show|status)
    shift; api GET "/api/plan-db/list" | python3 -m json.tool
    ;;
  health)
    api GET "/api/health" | python3 -m json.tool
    ;;
  *)
    echo "plan-db-api.sh — Curl wrapper for claude-core Plan DB API"
    echo ""
    echo "Commands: list, create, start, complete, cancel, approve,"
    echo "  update-task, json, get-context, execution-tree, drift-check,"
    echo "  validate-task, import, update-wave, kb-search,"
    echo "  agent-start, agent-complete, show, health"
    exit 1
    ;;
esac
