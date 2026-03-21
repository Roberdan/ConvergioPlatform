#!/usr/bin/env bash
# convergio-sync.sh — Cross-repo agent coordination
# Enables agents working in different repos to communicate and synchronize
# Usage: convergio-sync.sh <command> [args]
set -uo pipefail

PLATFORM_DIR="${CONVERGIO_PLATFORM_DIR:-$HOME/GitHub/ConvergioPlatform}"
DB="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"
BUS="$PLATFORM_DIR/scripts/platform/convergio-bus.sh"
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"

_db() { sqlite3 "$DB" "$1" 2>/dev/null; }

# ─── Repo Registry ──────────────────────────────────────────────────

cmd_register_repo() {
  local name="${1:?Usage: register-repo <name> <path>}"
  local path="${2:?Usage: register-repo <name> <path>}"

  _db "CREATE TABLE IF NOT EXISTS repo_registry (
    name TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    has_claude_config INTEGER DEFAULT 0,
    has_github_agents INTEGER DEFAULT 0,
    registered_at TEXT DEFAULT (datetime('now'))
  );"

  local has_claude=0 has_github=0
  [ -d "$path/.claude" ] && has_claude=1
  [ -d "$path/.github/agents" ] && has_github=1

  _db "INSERT OR REPLACE INTO repo_registry (name, path, has_claude_config, has_github_agents)
       VALUES ('$name', '$path', $has_claude, $has_github);"

  echo "Registered: $name → $path (claude:$has_claude, copilot:$has_github)"
}

cmd_list_repos() {
  echo "Registered repos:"
  _db "SELECT name, path, has_claude_config, has_github_agents FROM repo_registry ORDER BY name;" \
    | while IFS='|' read -r name path cc ga; do
    printf "  %-20s %s (claude:%s copilot:%s)\n" "$name" "$path" "$cc" "$ga"
  done
}

# ─── Cross-Repo Requests ────────────────────────────────────────────

cmd_request() {
  local from_repo="${1:?Usage: request <from-repo> <to-repo> <description>}"
  local to_repo="${2:?}"
  shift 2
  local description="$*"

  _db "CREATE TABLE IF NOT EXISTS cross_repo_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_repo TEXT NOT NULL,
    to_repo TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT DEFAULT 'pending' CHECK(status IN ('pending','accepted','in_progress','done','rejected')),
    assigned_agent TEXT,
    result TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    completed_at TEXT
  );"

  _db "INSERT INTO cross_repo_requests (from_repo, to_repo, description)
       VALUES ('$from_repo', '$to_repo', '$(echo "$description" | sed "s/'/''/g")');"

  local req_id
  req_id=$(_db "SELECT last_insert_rowid();")

  # Notify via bus
  "$BUS" broadcast "sync" "CROSS-REPO REQUEST #$req_id: $from_repo needs $to_repo: $description" 2>/dev/null || true

  echo "Request #$req_id created: $from_repo → $to_repo"
  echo "  $description"
}

cmd_pending() {
  local repo="${1:-}"
  echo "Pending cross-repo requests:"
  local query="SELECT id, from_repo, to_repo, description, status FROM cross_repo_requests WHERE status IN ('pending','accepted','in_progress')"
  [ -n "$repo" ] && query="$query AND to_repo = '$repo'"
  query="$query ORDER BY created_at DESC LIMIT 20;"

  _db "$query" | while IFS='|' read -r id from to desc status; do
    printf "  #%-4s %-12s → %-12s [%s] %s\n" "$id" "$from" "$to" "$status" "$desc"
  done
}

cmd_accept() {
  local req_id="${1:?Usage: accept <request-id> [agent-name]}"
  local agent="${2:-ali}"

  _db "UPDATE cross_repo_requests SET status='accepted', assigned_agent='$agent' WHERE id=$req_id;"
  local desc
  desc=$(_db "SELECT description FROM cross_repo_requests WHERE id=$req_id;")

  echo "Request #$req_id accepted, assigned to $agent"
  "$BUS" send "sync" "$agent" "ACCEPTED cross-repo request #$req_id: $desc" 2>/dev/null || true
}

cmd_complete() {
  local req_id="${1:?Usage: complete <request-id> <result>}"
  shift
  local result="$*"

  _db "UPDATE cross_repo_requests SET status='done', result='$(echo "$result" | sed "s/'/''/g")', completed_at=datetime('now') WHERE id=$req_id;"

  local from_repo
  from_repo=$(_db "SELECT from_repo FROM cross_repo_requests WHERE id=$req_id;")

  echo "Request #$req_id completed"
  "$BUS" broadcast "sync" "CROSS-REPO DONE #$req_id for $from_repo: $result" 2>/dev/null || true
}

# ─── Ali Auto-Dispatch for Cross-Repo ────────────────────────────────

cmd_auto_dispatch() {
  echo "=== Cross-Repo Auto-Dispatch ==="

  _db "SELECT id, from_repo, to_repo, description
       FROM cross_repo_requests
       WHERE status = 'pending'
       ORDER BY created_at ASC;" | while IFS='|' read -r id from to desc; do

    echo "  Processing #$id: $from → $to: $desc"

    # Find repo path
    local repo_path
    repo_path=$(_db "SELECT path FROM repo_registry WHERE name='$to';")

    if [ -z "$repo_path" ]; then
      echo "    SKIP: repo '$to' not registered"
      continue
    fi

    # Auto-accept and dispatch Ali in target repo
    _db "UPDATE cross_repo_requests SET status='in_progress', assigned_agent='ali' WHERE id=$id;"

    echo "    Dispatching Ali in $repo_path..."
    if command -v claude &>/dev/null; then
      (cd "$repo_path" && claude -p "Sei Ali. Cross-repo request #$id da $from: $desc. Risolvi e poi esegui: convergio-sync.sh complete $id 'risultato'" &)
      echo "    Ali spawned in $to"
    else
      echo "    Claude CLI not found — manual dispatch needed"
    fi
  done
}

case "${1:-help}" in
  register-repo)  shift; cmd_register_repo "$@" ;;
  repos|list)     cmd_list_repos ;;
  request|req)    shift; cmd_request "$@" ;;
  pending)        shift; cmd_pending "${1:-}" ;;
  accept)         shift; cmd_accept "$@" ;;
  complete|done)  shift; cmd_complete "$@" ;;
  auto-dispatch)  cmd_auto_dispatch ;;
  *)
    echo "convergio-sync.sh — Cross-repo agent coordination"
    echo ""
    echo "  register-repo <name> <path>        Register a repo"
    echo "  repos                              List registered repos"
    echo "  request <from> <to> <description>  Create cross-repo request"
    echo "  pending [repo]                     Show pending requests"
    echo "  accept <id> [agent]                Accept and assign"
    echo "  complete <id> <result>             Mark done with result"
    echo "  auto-dispatch                      Ali auto-processes pending requests"
    ;;
esac
