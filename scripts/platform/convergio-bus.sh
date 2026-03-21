#!/usr/bin/env bash
# convergio-bus.sh — Agent message bus (daemon API client)
# All operations go through the daemon HTTP API on :8420
# No file-based messaging — daemon is the single source of truth
set -uo pipefail

DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"

_curl() {
  curl -sf --connect-timeout 2 "$@" 2>/dev/null
}

_daemon_ok() {
  _curl "$DAEMON_URL/api/ipc/status" > /dev/null 2>&1
}

_require_daemon() {
  if ! _daemon_ok; then
    echo "ERROR: Daemon not running on $DAEMON_URL" >&2
    echo "Start with: ./daemon/start.sh" >&2
    return 1
  fi
}

cmd_register() {
  local name="${1:?Usage: register <name> [role] [tool]}"
  local role="${2:-agent}"
  local tool="${3:-claude}"
  _require_daemon || return 1

  local resp
  resp=$(_curl -X POST "$DAEMON_URL/api/ipc/agents/register" \
    -H "Content-Type: application/json" \
    -d "{\"agent_id\":\"$name\",\"host\":\"$(hostname -s)\",\"agent_type\":\"$tool\",\"pid\":$$,\"metadata\":\"{\\\"role\\\":\\\"$role\\\"}\"}")

  if [ $? -eq 0 ]; then
    echo "Registered: $name ($role via $tool)"
  else
    echo "ERROR: Registration failed" >&2
    return 1
  fi
}

cmd_unregister() {
  local name="${1:?Usage: unregister <name>}"
  _require_daemon || return 1

  _curl -X POST "$DAEMON_URL/api/ipc/agents/unregister" \
    -H "Content-Type: application/json" \
    -d "{\"agent_id\":\"$name\",\"host\":\"$(hostname -s)\"}" > /dev/null

  echo "Unregistered: $name"
}

cmd_who() {
  _require_daemon || return 1

  local resp
  resp=$(_curl "$DAEMON_URL/api/ipc/agents")
  if [ -z "$resp" ]; then
    echo "Agents: (none or daemon unavailable)"
    return 0
  fi

  echo "Agents:"
  echo "$resp" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    agents = d.get('agents', [])
    if not agents:
        print('  (nessuno)')
    else:
        for a in agents:
            name = a.get('name', '?')
            atype = a.get('agent_type', '?')
            host = a.get('host', '?')
            last = a.get('last_seen', '?')[:16]
            meta = a.get('metadata', '{}')
            try:
                role = json.loads(meta).get('role', atype)
            except:
                role = atype
            print(f'  {name:<12} {role:<24} {atype:<8} {host:<10} {last}')
except Exception as e:
    print(f'  Parse error: {e}', file=sys.stderr)
" 2>/dev/null || echo "  (parse error)"
}

cmd_send() {
  local from="${1:?Usage: send <from> <to> <message>}"
  local to="${2:?}"
  shift 2
  local msg="$*"
  _require_daemon || return 1

  # Use channel = "dm:{to}" for directed messages
  local channel="dm:${to}"
  local content="${from}: ${msg}"

  _curl -X POST "$DAEMON_URL/api/ipc/send" \
    -H "Content-Type: application/json" \
    -d "{\"channel\":\"$channel\",\"sender_name\":\"$from\",\"content\":\"$content\"}" > /dev/null

  echo "[$from -> $to] $msg"
}

cmd_read() {
  local name="${1:?Usage: read <name>}"
  _require_daemon || return 1

  local channel="dm:${name}"
  local resp
  resp=$(_curl "$DAEMON_URL/api/ipc/messages?channel=$channel&limit=20")

  if [ -z "$resp" ]; then
    echo "(nessun messaggio per $name)"
    return 0
  fi

  echo "$resp" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    msgs = d.get('messages', [])
    if not msgs:
        print(f'(nessun messaggio per \"$name\")')
    else:
        for m in msgs:
            content = m.get('content', '')
            time = m.get('created_at', '?')[:19]
            sender = m.get('from_agent', '?')
            print(f'[{time}] {content}')
except Exception as e:
    print(f'Parse error: {e}', file=sys.stderr)
" 2>/dev/null || echo "  (parse error)"
}

cmd_broadcast() {
  local from="${1:?Usage: broadcast <from> <message>}"
  shift
  local msg="$*"
  _require_daemon || return 1

  _curl -X POST "$DAEMON_URL/api/ipc/send" \
    -H "Content-Type: application/json" \
    -d "{\"channel\":\"general\",\"sender_name\":\"$from\",\"content\":\"$msg\"}" > /dev/null

  echo "[broadcast] $from: $msg"
}

cmd_channels() {
  _require_daemon || return 1
  _curl "$DAEMON_URL/api/ipc/channels" | python3 -m json.tool 2>/dev/null
}

cmd_status() {
  _require_daemon || return 1
  _curl "$DAEMON_URL/api/ipc/status" | python3 -m json.tool 2>/dev/null
}

cmd_models() {
  _require_daemon || return 1
  _curl "$DAEMON_URL/api/ipc/models" | python3 -m json.tool 2>/dev/null
}

cmd_skills() {
  _require_daemon || return 1
  _curl "$DAEMON_URL/api/ipc/skills" | python3 -m json.tool 2>/dev/null
}

cmd_metrics() {
  _require_daemon || return 1
  _curl "$DAEMON_URL/api/ipc/metrics" | python3 -m json.tool 2>/dev/null
}

case "${1:-help}" in
  register)    shift; cmd_register "$@" ;;
  unregister)  shift; cmd_unregister "$@" ;;
  who)         cmd_who ;;
  send)        shift; cmd_send "$@" ;;
  read)        shift; cmd_read "$@" ;;
  broadcast)   shift; cmd_broadcast "$@" ;;
  channels)    cmd_channels ;;
  status)      cmd_status ;;
  models)      cmd_models ;;
  skills)      cmd_skills ;;
  metrics)     cmd_metrics ;;
  help|*)
    echo "convergio-bus.sh — Agent message bus (daemon API client)"
    echo ""
    echo "  register <name> [role] [tool]  Register agent with daemon"
    echo "  unregister <name>              Remove agent"
    echo "  who                            List active agents"
    echo "  send <from> <to> <message>     Send directed message"
    echo "  read <name>                    Read messages for agent"
    echo "  broadcast <from> <message>     Broadcast to general channel"
    echo "  channels                       List channels"
    echo "  status                         IPC engine status"
    echo "  models                         Available models"
    echo "  skills                         Agent skill pool"
    echo "  metrics                        System metrics"
    echo ""
    echo "  Daemon: $DAEMON_URL"
    ;;
esac
