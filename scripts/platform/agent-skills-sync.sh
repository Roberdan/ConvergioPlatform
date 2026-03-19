#!/usr/bin/env bash
# agent-skills-sync.sh — Sync skills from claude-config/commands/ to IPC skill pool
# Usage: agent-skills-sync.sh [--platform-dir /path/to/ConvergioPlatform]
set -euo pipefail

# --- Config ---
CONVERGIO_DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"
DASHBOARD_DB="${DASHBOARD_DB:-$HOME/.claude/data/dashboard.db}"
_CURL_OPTS=(-s --max-time 3 -H 'Content-Type: application/json')

# --- Skill-to-agent mapping (bash 3.2 compatible) ---
# Format: "skill:agent:confidence" per line
_SKILL_MAP="planner:claude-opus:0.95
execute:copilot-codex:0.90
check:claude-sonnet:0.90
prompt:claude-opus:0.95
research:claude-haiku:0.80
release:claude-sonnet:0.85
prepare:claude-opus:0.90
optimize-instructions:claude-sonnet:0.85"

_lookup_agent() {
  local skill="$1" default="$2" result
  result=$(echo "$_SKILL_MAP" | while IFS=: read -r s a _c; do
    [[ "$s" == "$skill" ]] && echo "$a"
  done)
  echo "${result:-$default}"
}

_lookup_confidence() {
  local skill="$1" default="$2" result
  result=$(echo "$_SKILL_MAP" | while IFS=: read -r s _a c; do
    [[ "$s" == "$skill" ]] && echo "$c"
  done)
  echo "${result:-$default}"
}

# --- Resolve platform dir ---
_resolve_platform_dir() {
  local dir=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --platform-dir) dir="${2:?--platform-dir requires a path}"; shift 2 ;;
      *) shift ;;
    esac
  done
  if [[ -z "$dir" ]]; then
    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    dir="$(cd "$script_dir/../.." && pwd)"
  fi
  echo "$dir"
}

# --- Register a skill via CLI or HTTP fallback ---
_register_skill() {
  local skill="$1" agent="$2" confidence="$3"
  local payload
  payload='{"source":"sync","agent":"'"$agent"'","confidence":'"$confidence"'}'

  if command -v claude-core >/dev/null 2>&1; then
    claude-core ipc-intel request-skill "$skill" --payload "$payload" 2>/dev/null && return 0
  fi
  local body='{"sender_name":"skills-sync","channel":"skills","content":"register:'"$skill"':'"$agent"':'"$confidence"'"}'
  curl "${_CURL_OPTS[@]}" -X POST "${CONVERGIO_DAEMON_URL}/api/ipc/send" -d "$body" >/dev/null 2>&1 || true
}

# --- Main ---
main() {
  local platform_dir
  platform_dir="$(_resolve_platform_dir "$@")"
  local commands_dir="$platform_dir/claude-config/commands"
  local agents_dir="$platform_dir/claude-config/agents"

  if [[ ! -d "$commands_dir" ]]; then
    echo "Error: commands dir not found: $commands_dir" >&2
    exit 1
  fi

  local skill_count=0
  local agent_count=0
  local seen_agents=""

  printf "%-24s %-18s %s\n" "SKILL" "AGENT" "CONFIDENCE"

  for file in "$commands_dir"/*.md; do
    [[ -f "$file" ]] || continue
    local skill agent confidence
    skill="$(basename "$file" .md)"
    agent="$(_lookup_agent "$skill" "claude-sonnet")"
    confidence="$(_lookup_confidence "$skill" "0.80")"

    _register_skill "$skill" "$agent" "$confidence"
    printf "%-24s %-18s %s\n" "$skill" "$agent" "$confidence"
    skill_count=$((skill_count + 1))

    if [[ "$seen_agents" != *"|$agent|"* ]]; then
      seen_agents="${seen_agents}|$agent|"
      agent_count=$((agent_count + 1))
    fi
  done

  if [[ -d "$agents_dir" ]]; then
    for file in "$agents_dir"/*.md; do
      [[ -f "$file" ]] || continue
      local agent_name
      agent_name="$(basename "$file" .md)"
      if [[ "$seen_agents" != *"|$agent_name|"* ]]; then
        seen_agents="${seen_agents}|$agent_name|"
        agent_count=$((agent_count + 1))
      fi
    done
  fi

  echo ""
  echo "Synced $skill_count skills, $agent_count agents"
}

main "$@"
