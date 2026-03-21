#!/usr/bin/env bash
# convergio-run-ops.sh — Run lifecycle: solve, stop, pause, resume + cleanup
# Sourced by convergio. PLATFORM_DIR, BUS, DAEMON_URL, color vars must be set.
set -euo pipefail

_cleanup_agent() {
  local name="${CONVERGIO_AGENT_NAME:-}"

  # Kill entire process tree (all children, grandchildren, etc.)
  local pgid
  pgid=$(ps -o pgid= $$ 2>/dev/null | tr -d ' ')
  if [ -n "$pgid" ] && [ "$pgid" != "0" ]; then
    # Kill all processes in our process group except ourselves
    kill -- -"$pgid" 2>/dev/null || true
  fi

  # Also kill any claude/copilot processes we spawned
  pkill -P $$ 2>/dev/null || true

  # Unregister from daemon
  [ -n "$name" ] && "$BUS" unregister "$name" 2>/dev/null

  # Broadcast abort so other agents know
  "$BUS" broadcast "${name:-system}" "ABORT: $name shutdown" 2>/dev/null || true

  echo -e "\n${Y}Agent $name stopped. All child processes killed.${N}"
}

cmd_solve() {
  # Parse flags
  local autonomy="approve-plan"  # default: approve plan, then auto-execute
  local goal_parts=()
  for arg in "$@"; do
    case "$arg" in
      --autonomous)    autonomy="autonomous" ;;
      --approve-each)  autonomy="approve-each" ;;
      --approve-plan)  autonomy="approve-plan" ;;
      *)               goal_parts+=("$arg") ;;
    esac
  done
  local goal="${goal_parts[*]}"
  [ -z "$goal" ] && {
    echo -e "${R}Usage: convergio solve \"problem\" [--autonomous|--approve-plan|--approve-each]${N}"
    return 1
  }

  export CONVERGIO_AGENT_NAME="ali"
  "$TOGGLE" on 2>/dev/null
  "$BUS" register "ali" "orchestrator" "claude" 2>/dev/null || true

  echo -e "${B}Convergio Solve${N}"
  echo -e "${D}Goal: $goal${N}"
  echo -e "${D}Autonomy: $autonomy${N}"
  echo -e "${G}Spawning Ali (Chief of Staff, Opus)...${N}\n"

  local p="Sei Ali, il Chief of Staff di Convergio. "
  p+="PROBLEMA: $goal "
  p+="MODALITA: $autonomy "
  case "$autonomy" in
    autonomous)   p+="Esegui tutto senza chiedere approvazione. Solo escalate su 3x failure. " ;;
    approve-plan) p+="Presenta il piano per approvazione, poi esegui tutto automaticamente. Ferma solo pre-merge. " ;;
    approve-each) p+="Chiedi approvazione PRIMA di ogni dispatch di agente e PRIMA di ogni merge. " ;;
  esac
  p+="ISTRUZIONI: "
  p+="1) Analizza il problema — dominio, complessita, ruoli. "
  p+="2) Cerca agenti: sqlite3 \$DASHBOARD_DB \"SELECT ac.name, ac.category, ac.model, ask.confidence, substr(ac.description,1,60) FROM agent_catalog ac JOIN agent_skills ask ON ac.name=ask.agent_name WHERE ask.skill IN ('keyword') ORDER BY ask.confidence DESC;\" "
  p+="3) Assembla team (confidence-weighted). "
  p+="4) Invoca /planner per il piano (SEMPRE per 3+ task). "
  p+="5) Per ogni task, scegli il validatore corretto per output_type (Thor per code, doc-validator per documenti, strategy-validator per analisi, design-validator per design, compliance-validator per legal). "
  p+="6) Dispatcha agenti. Passa output tra agenti via shared context. "
  p+="7) Monitora, re-dispatcha su failure (max 3 tentativi per task). "
  p+="8) Riporta risultati con metriche. "
  p+="NON implementare tu — delega SEMPRE agli specialisti."

  _launch_tool "claude" "$p"
  "$BUS" unregister "ali" 2>/dev/null || true
}

cmd_stop() {
  local run_id="${1:-}"
  echo -e "${Y}Stopping Convergio processes (NOT your other work)...${N}"

  # 1. Broadcast abort to all agents via daemon
  "$BUS" broadcast "system" "ABORT: User requested full stop" 2>/dev/null || true

  # 2. Kill ONLY convergio-spawned processes (not user's own Claude/Copilot)
  pkill -f "convergio-autopilot" 2>/dev/null && echo "  Killed: autopilot"

  # 3. Kill agent sessions — identified by CONVERGIO_AGENT_NAME env var
  #    Safe: only processes launched via `convergio <agent>` have this env
  ps -eo pid,command 2>/dev/null | grep "CONVERGIO_AGENT_NAME" | grep -v grep | while read pid rest; do
    kill "$pid" 2>/dev/null && echo "  Killed: agent PID $pid"
  done

  # 4. Cancel active execution runs in DB
  local db="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"
  if [ -n "$run_id" ]; then
    sqlite3 "$db" "UPDATE execution_runs SET status='cancelled', completed_at=datetime('now') WHERE id=$run_id;" 2>/dev/null
    echo "  Run #$run_id cancelled"
  else
    local active
    active=$(sqlite3 "$db" "SELECT count(*) FROM execution_runs WHERE status='running';" 2>/dev/null)
    if [ "${active:-0}" -gt 0 ]; then
      sqlite3 "$db" "UPDATE execution_runs SET status='cancelled', completed_at=datetime('now') WHERE status='running';" 2>/dev/null
      echo "  Cancelled $active running execution(s)"
    fi
  fi

  # 5. Unregister all convergio agents from daemon (NOT the daemon itself)
  if _daemon_ok; then
    curl -sf "$DAEMON_URL/api/ipc/agents" 2>/dev/null | python3 -c "
import sys, json
try:
    for a in json.load(sys.stdin).get('agents', []):
        print(a.get('name', ''))
except: pass
" 2>/dev/null | while read name; do
      [ -n "$name" ] && "$BUS" unregister "$name" 2>/dev/null
    done
    echo "  Agents unregistered from daemon"
  fi

  echo -e "${G}Convergio stopped. Daemon still running (use 'convergio daemon stop' to stop daemon).${N}"
}

cmd_pause() {
  local run_id="${1:-}"
  [ -z "$run_id" ] && { echo -e "${R}Usage: convergio pause <run_id>${N}"; return 1; }

  local db="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"
  sqlite3 "$db" "UPDATE execution_runs SET status='paused', paused_at=datetime('now') WHERE id=$run_id;" 2>/dev/null
  "$BUS" broadcast "system" "PAUSE: run $run_id paused by user" 2>/dev/null || true
  echo -e "${Y}Run #$run_id paused.${N}"
}

cmd_resume() {
  local run_id="${1:-}"
  [ -z "$run_id" ] && { echo -e "${R}Usage: convergio resume <run_id>${N}"; return 1; }

  local db="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"
  sqlite3 "$db" "UPDATE execution_runs SET status='running', paused_at=NULL WHERE id=$run_id;" 2>/dev/null
  "$BUS" broadcast "system" "RESUME: run $run_id resumed by user" 2>/dev/null || true
  echo -e "${G}Run #$run_id resumed.${N}"
}

_launch_tool() {
  local tool="$1" prompt="$2"
  export CONVERGIO_AGENT_NAME="${CONVERGIO_AGENT_NAME:-agent}"

  # Trap Ctrl+C for clean shutdown
  trap '_cleanup_agent; exit 0' INT TERM

  case "$tool" in
    claude)
      if command -v claude &>/dev/null; then
        claude "$prompt"
      else
        echo -e "${Y}Claude CLI not found${N}"; echo "$prompt"
      fi ;;
    copilot)
      command -v gh &>/dev/null && { echo "$prompt"; gh copilot 2>/dev/null; } \
        || { echo -e "${Y}gh CLI not found${N}"; echo "$prompt"; } ;;
    opencode)
      command -v opencode &>/dev/null && { echo "$prompt" | opencode 2>/dev/null; } \
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
        echo -e "${Y}Local LLM not running. Start with: convergio-llm.sh start${N}"
        echo "$prompt"
      fi ;;
    *) echo -e "${Y}Unknown tool: $tool${N}"; echo "$prompt" ;;
  esac
}

cmd_launch() {
  local agent="$1" custom_name="${2:-}" custom_tool="${3:-}"
  local entry
  entry=$(get_agent "$agent") || { echo -e "${R}Unknown: $agent${N}" >&2; return 1; }

  IFS='|' read -r name desc invoke model default_tool <<< "$entry"
  local sname="${custom_name:-$name}"
  local tool="${custom_tool:-$default_tool}"

  "$TOGGLE" on 2>/dev/null
  "$BUS" register "$sname" "$desc" "$tool" 2>/dev/null || true
  export CONVERGIO_AGENT_NAME="$sname"

  echo -e "${G}Launching ${B}$sname${N}${G} ($desc)${N}"
  echo -e "${D}Model: $model | Tool: $tool | Invoke: $invoke${N}\n"

  local p="Il tuo nome è ${sname}. Ruolo: ${desc}. "
  p+="MESSAGING: I messaggi in arrivo ti vengono mostrati automaticamente. "
  p+="Per inviare: convergio-bus.sh send ${sname} <destinatario> <messaggio>. "
  p+="Per vedere chi è online: convergio-bus.sh who. "
  [[ "$invoke" == /* ]] && p+="Esegui $invoke per iniziare." || p+="Lavora secondo il tuo ruolo."

  _launch_tool "$tool" "$p"
  "$BUS" unregister "$sname" 2>/dev/null || true
}

cmd_named() {
  local sname="${1:?Usage: convergio as <name> [--tool T]}"
  local tool="${2:-claude}"
  export CONVERGIO_AGENT_NAME="$sname"
  "$TOGGLE" on 2>/dev/null
  "$BUS" register "$sname" "custom agent" "$tool" 2>/dev/null || true
  echo -e "${G}Launching ${B}$sname${N}\n"
  local p="Il tuo nome è ${sname}. Sei un agente Convergio. Usa convergio-bus.sh per comunicare."
  _launch_tool "$tool" "$p"
  "$BUS" unregister "$sname" 2>/dev/null || true
}

cmd_menu() {
  echo -e "${B}Convergio — Select Agent${N}\n"
  local i=1 names=()
  for entry in "${AGENT_LIST[@]}"; do
    IFS='|' read -r name desc invoke model tool <<< "$entry"
    printf "  ${C}%2d${N}. %-12s %s ${D}(%s/%s)${N}\n" "$i" "$name" "$desc" "$model" "$tool"
    names+=("$name")
    i=$((i + 1))
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
