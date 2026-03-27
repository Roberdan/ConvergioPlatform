#!/usr/bin/env bash
# convergio-sync.sh — Cross-repo agent coordination
# Enables agents working in different repos to communicate and synchronize
# Usage: convergio-sync.sh <command> [args]
set -euo pipefail

PLATFORM_DIR="${CONVERGIO_PLATFORM_DIR:-$HOME/GitHub/ConvergioPlatform}"
BUS="$PLATFORM_DIR/scripts/platform/convergio-bus.sh"
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }
command -v curl &>/dev/null || { echo "ERROR: curl required" >&2; exit 1; }

_api_get() { curl -sf --connect-timeout 2 "${DAEMON_URL}${1}" 2>/dev/null; }
_api_post() { curl -sf -X POST "${DAEMON_URL}${1}" -H 'Content-Type: application/json' -d "$2" 2>/dev/null; }

# ─── Repo Registry ──────────────────────────────────────────────────

cmd_register_repo() {
  local name="${1:?Usage: register-repo <name> <path>}"
  local path="${2:?Usage: register-repo <name> <path>}"

  local has_claude=0 has_github=0
  [ -d "$path/.claude" ] && has_claude=1
  [ -d "$path/.github/agents" ] && has_github=1

  # Use cvg repo add for registration
  cvg repo add "$name" --path "$path" 2>/dev/null || {
    # Fallback: daemon API for repo_registry table
    # TODO: needs daemon endpoint for repo_registry CRUD
    _api_post "/api/plan-db/repo-register" \
      "{\"name\":\"${name}\",\"path\":\"${path}\",\"has_claude_config\":${has_claude},\"has_github_agents\":${has_github}}" || {
      echo "ERROR: failed to register repo" >&2
      return 1
    }
  }

  echo "Registered: $name -> $path (claude:$has_claude, copilot:$has_github)"
}

cmd_list_repos() {
  echo "Registered repos:"
  # Use cvg repo list
  cvg repo list 2>/dev/null | while IFS= read -r line; do
    echo "  $line"
  done
}

# ─── Cross-Repo Requests ────────────────────────────────────────────

cmd_request() {
  local from_repo="${1:?Usage: request <from-repo> <to-repo> <description>}"
  local to_repo="${2:?}"
  shift 2
  local description="$*"

  local result
  # TODO: needs daemon endpoint for cross_repo_requests CRUD
  result=$(_api_post "/api/plan-db/cross-repo-request" \
    "{\"from_repo\":\"${from_repo}\",\"to_repo\":\"${to_repo}\",\"description\":\"${description}\"}") || {
    echo "ERROR: failed to create cross-repo request" >&2
    return 1
  }

  local req_id
  req_id=$(echo "$result" | jq -r '.id // empty' 2>/dev/null || echo "")

  # Notify via bus
  "$BUS" broadcast "sync" "CROSS-REPO REQUEST #$req_id: $from_repo needs $to_repo: $description" 2>/dev/null || true

  echo "Request #${req_id:-?} created: $from_repo -> $to_repo"
  echo "  $description"
}

cmd_pending() {
  local repo="${1:-}"
  echo "Pending cross-repo requests:"

  local url="/api/plan-db/cross-repo-requests?status=pending,accepted,in_progress"
  [ -n "$repo" ] && url="${url}&to_repo=${repo}"

  # TODO: needs daemon endpoint for cross_repo_requests listing
  local json
  json=$(_api_get "$url") || { echo "  (no data or daemon unavailable)" ; return 0; }

  echo "$json" | jq -r '.requests[]? | [(.id // ""), (.from_repo // ""), (.to_repo // ""), (.status // ""), (.description // "")] | @tsv' 2>/dev/null | while IFS=$'\t' read -r id from to status desc; do
    printf "  #%-4s %-12s -> %-12s [%s] %s\n" "$id" "$from" "$to" "$status" "$desc"
  done
}

cmd_accept() {
  local req_id="${1:?Usage: accept <request-id> [agent-name]}"
  local agent="${2:-ali}"

  [[ "$req_id" =~ ^[0-9]+$ ]] || { echo "error: request id must be numeric" >&2; exit 1; }

  # TODO: needs daemon endpoint for cross_repo_requests update
  _api_post "/api/plan-db/cross-repo-request/update" \
    "{\"id\":${req_id},\"status\":\"accepted\",\"assigned_agent\":\"${agent}\"}" || {
    echo "ERROR: failed to accept request" >&2
    return 1
  }

  echo "Request #$req_id accepted, assigned to $agent"

  local desc
  desc=$(_api_get "/api/plan-db/cross-repo-request/${req_id}" | jq -r '.description // ""' 2>/dev/null || echo "")
  "$BUS" send "sync" "$agent" "ACCEPTED cross-repo request #$req_id: $desc" 2>/dev/null || true
}

cmd_complete() {
  local req_id="${1:?Usage: complete <request-id> <result>}"
  shift
  local result="$*"

  [[ "$req_id" =~ ^[0-9]+$ ]] || { echo "error: request id must be numeric" >&2; exit 1; }

  # TODO: needs daemon endpoint for cross_repo_requests update
  _api_post "/api/plan-db/cross-repo-request/update" \
    "{\"id\":${req_id},\"status\":\"done\",\"result\":\"${result}\"}" || {
    echo "ERROR: failed to complete request" >&2
    return 1
  }

  local from_repo
  from_repo=$(_api_get "/api/plan-db/cross-repo-request/${req_id}" | jq -r '.from_repo // ""' 2>/dev/null || echo "")

  echo "Request #$req_id completed"
  "$BUS" broadcast "sync" "CROSS-REPO DONE #$req_id for $from_repo: $result" 2>/dev/null || true
}

# ─── Ali Auto-Dispatch for Cross-Repo ────────────────────────────────

cmd_auto_dispatch() {
  echo "=== Cross-Repo Auto-Dispatch ==="

  # TODO: needs daemon endpoint for cross_repo_requests listing
  local json
  json=$(_api_get "/api/plan-db/cross-repo-requests?status=pending") || { echo "  No pending requests"; return 0; }

  echo "$json" | jq -r '.requests[]? | [(.id // ""), (.from_repo // ""), (.to_repo // ""), (.description // "")] | @tsv' 2>/dev/null | while IFS=$'\t' read -r id from to desc; do
    [[ -z "$id" ]] && continue
    echo "  Processing #$id: $from -> $to: $desc"

    # Find repo path via cvg repo show
    local repo_path
    repo_path=$(cvg repo show "$to" 2>/dev/null | jq -r '.path // empty' 2>/dev/null || echo "")

    if [ -z "$repo_path" ]; then
      echo "    SKIP: repo '$to' not registered"
      continue
    fi

    # Auto-accept and dispatch Ali in target repo
    _api_post "/api/plan-db/cross-repo-request/update" \
      "{\"id\":${id},\"status\":\"in_progress\",\"assigned_agent\":\"ali\"}" 2>/dev/null || true

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
