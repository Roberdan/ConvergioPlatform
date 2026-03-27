#!/bin/bash
# idea-jar.sh — Capture, elaborate, promote ideas
# Version: 1.0.0
set -euo pipefail

export PATH="/opt/homebrew/bin:/usr/local/bin:$PATH"
DAEMON_API="http://localhost:8420"

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

# Colors
BOLD=$(tput bold 2>/dev/null || true)
RESET=$(tput sgr0 2>/dev/null || true)
GREEN=$(tput setaf 2 2>/dev/null || true)
YELLOW=$(tput setaf 3 2>/dev/null || true)
CYAN=$(tput setaf 6 2>/dev/null || true)
RED=$(tput setaf 1 2>/dev/null || true)

usage() {
  cat <<EOF
${BOLD}idea-jar.sh${RESET} — Capture, elaborate, promote ideas

${BOLD}USAGE${RESET}
  idea-jar.sh <command> [options]

${BOLD}COMMANDS${RESET}
  add "title" [--desc "text"] [--tags "a,b"] [--priority P1] [--project id]
  list [--status draft] [--priority P0] [--limit 20]
  edit <id> [--title "new"] [--desc "new"] [--status ready] [--tags "x,y"]
  note <id> "note text"
  promote <id>
  delete <id> [--force]

${BOLD}PRIORITIES${RESET}  P0 (critical) P1 (high) P2 (normal, default) P3 (low)
${BOLD}STATUSES${RESET}    draft | ready | promoted | archived
EOF
}

status_color() {
  case "$1" in
    promoted) printf '%s' "${GREEN}" ;;
    ready)    printf '%s' "${CYAN}" ;;
    archived) printf '%s' "${YELLOW}" ;;
    *)        printf '%s' "" ;;
  esac
}

cmd_add() {
  local title="" desc="" tags="[]" priority="P2" project_id=""
  local positional=()
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --desc)     desc="$2";       shift 2 ;;
      --tags)     tags=$(printf '"%s"' "$2" | sed 's/,/","/g; s/^/[/; s/$/]/'); shift 2 ;;
      --priority) priority="$2";   shift 2 ;;
      --project)  project_id="$2"; shift 2 ;;
      *)          positional+=("$1"); shift ;;
    esac
  done
  [[ ${#positional[@]} -gt 0 ]] && title="${positional[0]}"
  [[ -z "$title" ]] && { echo "${RED}Error: title required${RESET}"; exit 1; }

  local result
  result="$(curl -sf -X POST "${DAEMON_API}/api/ideas" \
    -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg t "$title" --arg d "$desc" --argjson tags "$tags" --arg p "$priority" --arg pid "$project_id" \
      '{title:$t, description:$d, tags:$tags, priority:$p, project_id:$pid, status:"draft"}')" 2>/dev/null || echo '{}')"
  local id
  id="$(echo "$result" | jq -r '.id // "?"' 2>/dev/null)"
  echo "${GREEN}${BOLD}Added idea #${id}:${RESET} $title  ${YELLOW}[$priority]${RESET}"
}

cmd_list() {
  local status_filter="" priority_filter="" limit=20
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --status)   status_filter="$2";   shift 2 ;;
      --priority) priority_filter="$2"; shift 2 ;;
      --limit)    limit="$2";           shift 2 ;;
      *)          shift ;;
    esac
  done

  local all_ideas
  all_ideas="$(curl -sf "${DAEMON_API}/api/ideas" 2>/dev/null || echo '[]')"
  local rows
  rows="$(echo "$all_ideas" | jq -r --arg sf "$status_filter" --arg pf "$priority_filter" --argjson lim "$limit" '
    [.[] | select(($sf == "" or .status == $sf) and ($pf == "" or .priority == $pf))]
    | sort_by(.priority, .created_at) | reverse | .[:$lim][]
    | "\(.id)|\(.priority)|\(.status)|\(.title)"
  ' 2>/dev/null || echo '')"

  if [[ -z "$rows" ]]; then
    echo "${YELLOW}No ideas found.${RESET}"
    return
  fi

  printf "${BOLD}%-5s %-4s %-10s %s${RESET}\n" "ID" "PRI" "STATUS" "TITLE"
  printf '%s\n' "$(printf '%.0s-' {1..60})"
  while IFS='|' read -r id pri status title; do
    local sc; sc=$(status_color "$status")
    printf "%-5s ${YELLOW}%-4s${RESET} ${sc}%-10s${RESET} %s\n" "$id" "$pri" "$status" "$title"
  done <<< "$rows"
}

cmd_edit() {
  local id="${1:-}"; shift || true
  [[ -z "$id" ]] && { echo "${RED}Error: id required${RESET}"; exit 1; }

  local updates='{}'
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --title)  updates="$(echo "$updates" | jq --arg v "$2" '. + {title:$v}')"; shift 2 ;;
      --desc)   updates="$(echo "$updates" | jq --arg v "$2" '. + {description:$v}')"; shift 2 ;;
      --status) updates="$(echo "$updates" | jq --arg v "$2" '. + {status:$v}')"; shift 2 ;;
      --tags)   local t; t=$(printf '"%s"' "$2" | sed 's/,/","/g; s/^/[/; s/$/]/'); updates="$(echo "$updates" | jq --argjson v "$t" '. + {tags:$v}')"; shift 2 ;;
      *)        shift ;;
    esac
  done

  [[ "$updates" == "{}" ]] && { echo "${YELLOW}Nothing to update.${RESET}"; exit 0; }

  curl -sf -X POST "${DAEMON_API}/api/ideas/${id}" \
    -H 'Content-Type: application/json' \
    -d "$updates" 2>/dev/null || true
  echo "${GREEN}Updated idea #${id}${RESET}"
}

cmd_note() {
  local id="${1:-}"; shift || true
  local content="${1:-}"
  [[ -z "$id" || -z "$content" ]] && { echo "${RED}Error: id and note text required${RESET}"; exit 1; }

  curl -sf -X POST "${DAEMON_API}/api/ideas/${id}/notes" \
    -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg c "$content" '{content:$c}')" 2>/dev/null || true
  echo "${GREEN}Note added to idea #${id}${RESET}"
}

cmd_promote() {
  local id="${1:-}"
  [[ -z "$id" ]] && { echo "${RED}Error: id required${RESET}"; exit 1; }

  curl -sf -X POST "${DAEMON_API}/api/ideas/${id}" \
    -H 'Content-Type: application/json' \
    -d '{"status":"promoted"}' 2>/dev/null || true
  local json
  json="$(curl -sf "${DAEMON_API}/api/ideas" 2>/dev/null | jq --argjson id "$id" '[.[] | select(.id == $id)]' 2>/dev/null || echo '[]')"
  echo "${GREEN}${BOLD}Promoted idea #${id}${RESET}"
  echo ""
  echo "${CYAN}--- Idea JSON (for /planner input) ---${RESET}"
  echo "$json"
}

cmd_delete() {
  local id="${1:-}"; shift || true
  local force=false
  [[ "${1:-}" == "--force" ]] && force=true

  [[ -z "$id" ]] && { echo "${RED}Error: id required${RESET}"; exit 1; }

  local title
  title="$(curl -sf "${DAEMON_API}/api/ideas" 2>/dev/null | jq -r --argjson id "$id" '.[] | select(.id == $id) | .title // ""' 2>/dev/null || echo '')"
  [[ -z "$title" ]] && { echo "${RED}Idea #${id} not found.${RESET}"; exit 1; }

  if [[ "$force" == false ]]; then
    printf "Delete idea #%s: %s? [y/N] " "$id" "$title"
    read -r answer
    [[ "$answer" != "y" && "$answer" != "Y" ]] && { echo "Aborted."; exit 0; }
  fi

  # TODO: needs daemon endpoint for idea deletion
  curl -sf -X DELETE "${DAEMON_API}/api/ideas/${id}" 2>/dev/null || true
  echo "${YELLOW}Deleted idea #${id}: ${title}${RESET}"
}

# Main dispatch
CMD="${1:-}"
[[ -z "$CMD" || "$CMD" == "--help" || "$CMD" == "-h" ]] && { usage; exit 0; }
shift

case "$CMD" in
  add)     cmd_add "$@" ;;
  list)    cmd_list "$@" ;;
  edit)    cmd_edit "$@" ;;
  note)    cmd_note "$@" ;;
  promote) cmd_promote "$@" ;;
  delete)  cmd_delete "$@" ;;
  *)       echo "${RED}Unknown command: $CMD${RESET}"; usage; exit 1 ;;
esac
