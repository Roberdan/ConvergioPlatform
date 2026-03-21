#!/usr/bin/env bash
# convergio-run-ops.sh — Run lifecycle: solve, stop, pause, resume + cleanup
# Sourced by convergio. PLATFORM_DIR, BUS, DAEMON_URL, color vars must be set.
set -euo pipefail

_validate_id() { [[ "$1" =~ ^[0-9]+$ ]] || die "Invalid ID: $1 (must be numeric)"; }

# _api: wraps curl calls to http://localhost:8420 (DAEMON_URL); falls back to sqlite3.
_api() { local r; r=$(curl -sf "$@" 2>/dev/null) && echo "$r" && return 0
  echo "WARN: daemon not responding, using sqlite3 fallback" >&2; return 1; }

_cleanup_agent() {
  local name="${CONVERGIO_AGENT_NAME:-}" pgid
  pgid=$(ps -o pgid= $$ 2>/dev/null | tr -d ' ')
  [ -n "$pgid" ] && [ "$pgid" != "0" ] && kill -- -"$pgid" 2>/dev/null || true
  pkill -P $$ 2>/dev/null || true
  [ -n "$name" ] && "$BUS" unregister "$name" 2>/dev/null
  "$BUS" broadcast "${name:-system}" "ABORT: $name shutdown" 2>/dev/null || true
  echo -e "\n${Y}Agent $name stopped. All child processes killed.${N}"
}

_prepare_context() {
  # Ingest context sources into a run dir; print context dir path to stdout.
  local run_id="$1" goal="$2" autonomy="$3"; shift 3
  local context_sources=("$@")
  local db="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"
  local run_dir="$PLATFORM_DIR/data/runs/${run_id}"
  mkdir -p "${run_dir}/context" "${run_dir}/outputs"
  local started_at sources_json="[]"
  started_at=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  [ ${#context_sources[@]} -gt 0 ] && \
    sources_json=$(printf '"%s",' "${context_sources[@]}" | sed 's/,$//' | { echo -n "["; cat; echo -n "]"; })
  printf '{"id":"%s","goal":"%s","context_sources":%s,"autonomy":"%s","status":"running","started_at":"%s"}\n' \
    "$run_id" "$goal" "$sources_json" "$autonomy" "$started_at" > "${run_dir}/run.json"
  local ingest_sh="${SCRIPT_DIR:-$(dirname "${BASH_SOURCE[0]}")}/convergio-ingest.sh"
  for src in "${context_sources[@]}"; do
    if [ -f "$ingest_sh" ]; then
      "$ingest_sh" "$src" "${run_dir}/context/" 2>/dev/null \
        || echo -e "${Y}Warning: ingest failed for $src (skipped)${N}" >&2
    else
      echo -e "${Y}Warning: convergio-ingest.sh not found — $src skipped${N}" >&2
    fi
  done
  # Register run via daemon API; fallback to sqlite3 if daemon unavailable.
  local ge; ge=$(printf '%s' "$goal" | sed 's/"/\\"/g')
  local ctx="${run_dir}/context/"
  if ! _api -X POST "${DAEMON_URL:-http://localhost:8420}/api/runs" \
      -H 'Content-Type: application/json' \
      -d "{\"goal\":\"${ge}\",\"status\":\"running\",\"context_path\":\"${ctx}\"}" >/dev/null; then
    local gs; gs=$(printf '%s' "$goal" | sed "s/'/''/g")
    sqlite3 "$db" \
      "INSERT OR IGNORE INTO execution_runs (goal, status, context_path) VALUES ('${gs}', 'running', '${ctx}');" \
      2>/dev/null || true
  fi
  echo "${run_dir}/context"
}

cmd_solve() {
  local autonomy="approve-plan" goal_parts=() context_sources=() skip_next=0
  for arg in "$@"; do
    if [ "$skip_next" -eq 1 ]; then context_sources+=("$arg"); skip_next=0; continue; fi
    case "$arg" in
      --autonomous)    autonomy="autonomous" ;;
      --approve-each)  autonomy="approve-each" ;;
      --approve-plan)  autonomy="approve-plan" ;;
      --context)       skip_next=1 ;;
      --context=*)     context_sources+=("${arg#--context=}") ;;
      *)               goal_parts+=("$arg") ;;
    esac
  done
  local goal="${goal_parts[*]}"
  [ -z "$goal" ] && {
    echo -e "${R}Usage: convergio solve \"problem\" [--context <src>] [--autonomous|--approve-plan|--approve-each]${N}"
    return 1
  }
  local context_dir=""
  if [ ${#context_sources[@]} -gt 0 ]; then
    local run_id; run_id=$(date +"%Y%m%d-%H%M%S")
    echo -e "${D}Preparing context run: $run_id (${#context_sources[@]} source(s))${N}"
    context_dir=$(_prepare_context "$run_id" "$goal" "$autonomy" "${context_sources[@]}")
    echo -e "${G}Context ready: $context_dir${N}"
  fi
  export CONVERGIO_AGENT_NAME="ali"
  "$TOGGLE" on 2>/dev/null
  "$BUS" register "ali" "orchestrator" "claude" 2>/dev/null || true
  echo -e "${B}Convergio Solve${N}\n${D}Goal: $goal\nAutonomy: $autonomy${N}"
  echo -e "${G}Spawning Ali (Chief of Staff, Opus)...${N}\n"
  local p="Sei Ali, il Chief of Staff di Convergio. PROBLEMA: $goal MODALITA: $autonomy "
  [ -n "$context_dir" ] && p+="CONTEXT_DIR: $context_dir "
  case "$autonomy" in
    autonomous)   p+="Esegui tutto senza chiedere approvazione. Solo escalate su 3x failure. " ;;
    approve-plan) p+="Presenta il piano per approvazione, poi esegui tutto automaticamente. Ferma solo pre-merge. " ;;
    approve-each) p+="Chiedi approvazione PRIMA di ogni dispatch di agente e PRIMA di ogni merge. " ;;
  esac
  p+="ISTRUZIONI: 1) Analizza il problema — dominio, complessita, ruoli. "
  p+="2) Cerca agenti: sqlite3 \$DASHBOARD_DB \"SELECT ac.name, ac.category, ac.model, ask.confidence, substr(ac.description,1,60) FROM agent_catalog ac JOIN agent_skills ask ON ac.name=ask.agent_name WHERE ask.skill IN ('keyword') ORDER BY ask.confidence DESC;\" "
  p+="3) Assembla team (confidence-weighted). 4) Invoca /planner per il piano (SEMPRE per 3+ task). "
  p+="5) Per ogni task, scegli il validatore corretto (Thor=code, doc-validator=docs, strategy-validator=analisi, design-validator=design, compliance-validator=legal). "
  p+="6) Dispatcha agenti. Passa output via shared context. "
  p+="7) Monitora, re-dispatcha su failure (max 3 tentativi). 8) Riporta risultati con metriche. "
  p+="NON implementare tu — delega SEMPRE agli specialisti."
  _launch_tool "claude" "$p"
  "$BUS" unregister "ali" 2>/dev/null || true
}

cmd_stop() {
  local run_id="${1:-}" db="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"
  [ -n "$run_id" ] && _validate_id "$run_id"
  echo -e "${Y}Stopping Convergio processes (NOT your other work)...${N}"
  "$BUS" broadcast "system" "ABORT: User requested full stop" 2>/dev/null || true
  pkill -f "convergio-autopilot" 2>/dev/null && echo "  Killed: autopilot"
  # Safe: only convergio-spawned processes have CONVERGIO_AGENT_NAME in env
  ps -eo pid,command 2>/dev/null | grep "CONVERGIO_AGENT_NAME" | grep -v grep | while read -r pid rest; do
    kill "$pid" 2>/dev/null && echo "  Killed: agent PID $pid"
  done
  if [ -n "$run_id" ]; then
    _api -X PUT "${DAEMON_URL:-http://localhost:8420}/api/runs/$run_id" \
      -H 'Content-Type: application/json' -d '{"status":"cancelled"}' >/dev/null \
    || sqlite3 "$db" "UPDATE execution_runs SET status='cancelled', completed_at=datetime('now') WHERE id=$run_id;" 2>/dev/null
    echo "  Run #$run_id cancelled"
  else
    local active
    active=$(sqlite3 "$db" "SELECT count(*) FROM execution_runs WHERE status='running';" 2>/dev/null)
    if [ "${active:-0}" -gt 0 ]; then
      sqlite3 "$db" "UPDATE execution_runs SET status='cancelled', completed_at=datetime('now') WHERE status='running';" 2>/dev/null
      echo "  Cancelled $active running execution(s)"
    fi
  fi
  if _daemon_ok; then
    curl -sf "$DAEMON_URL/api/ipc/agents" 2>/dev/null | python3 -c "
import sys, json
try:
    for a in json.load(sys.stdin).get('agents', []): print(a.get('name', ''))
except: pass
" 2>/dev/null | while read -r name; do
      [ -n "$name" ] && "$BUS" unregister "$name" 2>/dev/null
    done
    echo "  Agents unregistered from daemon"
  fi
  echo -e "${G}Convergio stopped. Daemon still running (use 'convergio daemon stop' to stop daemon).${N}"
}

cmd_pause() {
  local run_id="${1:-}"
  [ -z "$run_id" ] && { echo -e "${R}Usage: convergio pause <run_id>${N}"; return 1; }
  _validate_id "$run_id"
  local db="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"
  _api -X POST "${DAEMON_URL:-http://localhost:8420}/api/runs/$run_id/pause" >/dev/null \
  || sqlite3 "$db" "UPDATE execution_runs SET status='paused', paused_at=datetime('now') WHERE id=$run_id;" 2>/dev/null
  "$BUS" broadcast "system" "PAUSE: run $run_id paused by user" 2>/dev/null || true
  echo -e "${Y}Run #$run_id paused.${N}"
}

cmd_resume() {
  local run_id="${1:-}"
  [ -z "$run_id" ] && { echo -e "${R}Usage: convergio resume <run_id>${N}"; return 1; }
  _validate_id "$run_id"
  local db="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"
  _api -X POST "${DAEMON_URL:-http://localhost:8420}/api/runs/$run_id/resume" >/dev/null \
  || sqlite3 "$db" "UPDATE execution_runs SET status='running', paused_at=NULL WHERE id=$run_id;" 2>/dev/null
  "$BUS" broadcast "system" "RESUME: run $run_id resumed by user" 2>/dev/null || true
  echo -e "${G}Run #$run_id resumed.${N}"
}

_launch_tool() {
  local tool="$1" prompt="$2"
  export CONVERGIO_AGENT_NAME="${CONVERGIO_AGENT_NAME:-agent}"
  trap '_cleanup_agent; exit 0' INT TERM
  case "$tool" in
    claude)   command -v claude &>/dev/null && claude "$prompt" \
                || { echo -e "${Y}Claude CLI not found${N}"; echo "$prompt"; } ;;
    copilot)  command -v gh &>/dev/null && { echo "$prompt"; gh copilot 2>/dev/null; } \
                || { echo -e "${Y}gh CLI not found${N}"; echo "$prompt"; } ;;
    opencode) command -v opencode &>/dev/null && { echo "$prompt" | opencode 2>/dev/null; } \
                || { echo -e "${Y}OpenCode not found${N}"; echo "$prompt"; } ;;
    local)
      local LITELLM_URL="${LITELLM_URL:-http://localhost:4000}"
      if curl -sf "$LITELLM_URL/health" > /dev/null 2>&1; then
        echo -e "${G}Sending to local LLM ($LITELLM_URL)${N}\n"
        curl -sf "$LITELLM_URL/chat/completions" \
          -H "Content-Type: application/json" \
          -d "{\"model\":\"local\",\"messages\":[{\"role\":\"system\",\"content\":\"$prompt\"},{\"role\":\"user\",\"content\":\"Start.\"}]}" \
          2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin)['choices'][0]['message']['content'])" 2>/dev/null \
          || echo -e "${R}Local LLM request failed${N}"
      else
        echo -e "${Y}Local LLM not running. Start with: convergio-llm.sh start${N}"; echo "$prompt"
      fi ;;
    *) echo -e "${Y}Unknown tool: $tool${N}"; echo "$prompt" ;;
  esac
}

cmd_launch() {
  local agent="$1" custom_name="${2:-}" custom_tool="${3:-}" entry
  entry=$(get_agent "$agent") || { echo -e "${R}Unknown: $agent${N}" >&2; return 1; }
  IFS='|' read -r name desc invoke model default_tool <<< "$entry"
  local sname="${custom_name:-$name}" tool="${custom_tool:-$default_tool}"
  "$TOGGLE" on 2>/dev/null
  "$BUS" register "$sname" "$desc" "$tool" 2>/dev/null || true
  export CONVERGIO_AGENT_NAME="$sname"
  echo -e "${G}Launching ${B}$sname${N}${G} ($desc)${N}"
  echo -e "${D}Model: $model | Tool: $tool | Invoke: $invoke${N}\n"
  local p="Il tuo nome è ${sname}. Ruolo: ${desc}. MESSAGING: I messaggi in arrivo ti vengono mostrati automaticamente. "
  p+="Per inviare: convergio-bus.sh send ${sname} <destinatario> <messaggio>. "
  p+="Per vedere chi è online: convergio-bus.sh who. "
  [[ "$invoke" == /* ]] && p+="Esegui $invoke per iniziare." || p+="Lavora secondo il tuo ruolo."
  _launch_tool "$tool" "$p"
  "$BUS" unregister "$sname" 2>/dev/null || true
}

cmd_named() {
  local sname="${1:?Usage: convergio as <name> [--tool T]}" tool="${2:-claude}"
  export CONVERGIO_AGENT_NAME="$sname"
  "$TOGGLE" on 2>/dev/null
  "$BUS" register "$sname" "custom agent" "$tool" 2>/dev/null || true
  echo -e "${G}Launching ${B}$sname${N}\n"
  _launch_tool "$tool" "Il tuo nome è ${sname}. Sei un agente Convergio. Usa convergio-bus.sh per comunicare."
  "$BUS" unregister "$sname" 2>/dev/null || true
}

cmd_menu() {
  echo -e "${B}Convergio — Select Agent${N}\n"
  local i=1 names=()
  for entry in "${AGENT_LIST[@]}"; do
    IFS='|' read -r name desc invoke model tool <<< "$entry"
    printf "  ${C}%2d${N}. %-12s %s ${D}(%s/%s)${N}\n" "$i" "$name" "$desc" "$model" "$tool"
    names+=("$name"); i=$((i + 1))
  done
  echo ""
  echo -ne "${B}Agent (number/name): ${N}"; read -r choice
  echo -ne "${B}Custom name (Enter=default): ${N}"; read -r cname
  echo -ne "${B}Tool (Enter=default): ${N}"; read -r ctool
  local selected
  if [[ "$choice" =~ ^[0-9]+$ ]] && (( choice >= 1 && choice <= ${#names[@]} )); then
    selected="${names[$choice]}"
  else
    selected="$choice"
  fi
  cmd_launch "$selected" "${cname:-}" "${ctool:-}"
}
