#!/usr/bin/env bash
# convergio-learnings.sh — Learning loop: detect patterns, promote to knowledge/skills
# Analyzes plan_learnings for recurring patterns and auto-promotes
set -euo pipefail

PLATFORM_DIR="${CONVERGIO_PLATFORM_DIR:-$HOME/GitHub/ConvergioPlatform}"
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }
command -v curl &>/dev/null || { echo "ERROR: curl required" >&2; exit 1; }

_api_get() { curl -sf --connect-timeout 2 "${DAEMON_URL}${1}" 2>/dev/null; }
_api_post() { curl -sf -X POST "${DAEMON_URL}${1}" -H 'Content-Type: application/json' -d "$2" 2>/dev/null; }

cmd_analyze() {
  echo "=== Learning Pattern Analysis ==="
  echo ""

  # TODO: needs daemon endpoint for plan_learnings aggregation queries
  local json
  json=$(_api_get "/api/plan-db/learnings/analysis") || { echo "  Daemon not reachable or endpoint not available"; return 0; }

  echo "--- Recurring learning categories ---"
  echo "$json" | jq -r '.categories[]? | [(.category // ""), (.severity // ""), (.occurrences // 0)] | @tsv' 2>/dev/null | while IFS=$'\t' read -r cat sev count; do
    printf "  %-20s %-10s %s occurrences\n" "$cat" "$sev" "$count"
  done

  echo ""
  echo "--- Most common learning titles ---"
  echo "$json" | jq -r '.common_titles[]? | [(.freq // 0), (.title // "")] | @tsv' 2>/dev/null | while IFS=$'\t' read -r freq title; do
    printf "  [%sx] %s\n" "$freq" "$title"
  done

  echo ""
  echo "--- Actionable learnings not yet acted on ---"
  echo "$json" | jq -r '.actionable[]? | [(.plan_id // ""), (.title // "")] | @tsv' 2>/dev/null | while IFS=$'\t' read -r pid title; do
    echo "  Plan $pid: $title"
  done
}

cmd_promote() {
  echo "=== Auto-Promoting Recurring Learnings ==="
  local promoted=0

  # Promote learnings to knowledge base via daemon API
  # TODO: needs daemon endpoint for plan_learnings promotion
  local json
  json=$(_api_get "/api/plan-db/learnings/promotable") || { echo "  Daemon not reachable or endpoint not available"; return 0; }

  echo "$json" | jq -r '.promotable[]? | [(.title // ""), (.category // ""), (.freq // 0), (.plans // "")] | @tsv' 2>/dev/null | while IFS=$'\t' read -r title cat freq plans; do
    [[ -z "$title" ]] && continue

    # Write to knowledge base via cvg kb
    cvg kb write --category "$cat" --content "Auto-promoted from $freq plan learnings: $title" 2>/dev/null || \
      _api_post "/api/plan-db/kb-write" "{\"domain\":\"${cat}\",\"title\":\"${title}\",\"content\":\"Auto-promoted from ${freq} plan learnings\",\"source_type\":\"learned\",\"source_ref\":\"Plans: ${plans}\"}" 2>/dev/null || true

    echo "  PROMOTED: $title (${freq}x across plans $plans)"
    promoted=$((promoted + 1))
  done

  # Promote high-hit KB entries to earned_skills
  # TODO: needs daemon endpoint for knowledge_base promotion to skills
  local kb_json
  kb_json=$(_api_get "/api/plan-db/kb/promotable-skills") || kb_json=""

  if [[ -n "$kb_json" ]]; then
    echo "$kb_json" | jq -r '.promotable[]? | [(.id // ""), (.domain // ""), (.title // ""), (.content // ""), (.hit_count // 0)] | @tsv' 2>/dev/null | while IFS=$'\t' read -r id domain title content hits; do
      [[ -z "$id" ]] && continue
      local skill_name
      skill_name=$(echo "$title" | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | tr -cd 'a-z0-9-' | cut -c1-50)

      _api_post "/api/plan-db/skill/promote" "{\"name\":\"${skill_name}\",\"domain\":\"${domain}\",\"content\":\"${content}\",\"hit_count\":${hits},\"kb_id\":${id}}" 2>/dev/null || true
      echo "  SKILL: $skill_name (from KB entry with $hits hits)"
    done
  fi

  echo "  Promoted: $promoted learnings to knowledge base"
}

cmd_calibrate() {
  echo "=== Estimation Calibration ==="
  cvg plan calibrate-estimates 2>/dev/null || {
    echo "  No calibration data yet"
  }
}

cmd_summary() {
  echo "=== Knowledge System Status ==="
  local json
  json=$(_api_get "/api/metrics/summary") || { echo "  Daemon not reachable"; return 0; }

  # Get counts from overview/metrics
  local overview
  overview=$(_api_get "/api/overview") || overview=""

  local kb_count learning_count skill_count agent_count earned_count
  kb_count=$(echo "$overview" | jq -r '.knowledge_base_count // "?"' 2>/dev/null || echo "?")
  learning_count=$(echo "$overview" | jq -r '.plan_learnings_count // "?"' 2>/dev/null || echo "?")
  skill_count=$(echo "$overview" | jq -r '.agent_skills_count // "?"' 2>/dev/null || echo "?")
  earned_count=$(echo "$overview" | jq -r '.earned_skills_count // "?"' 2>/dev/null || echo "?")
  agent_count=$(echo "$overview" | jq -r '.agent_catalog_count // "?"' 2>/dev/null || echo "?")

  echo "  Knowledge base: $kb_count entries"
  echo "  Plan learnings: $learning_count entries"
  echo "  Agent skills:   $skill_count mappings"
  echo "  Earned skills:  $earned_count skills"
  echo "  Agent catalog:  $agent_count agents"
}

case "${1:-summary}" in
  analyze)   cmd_analyze ;;
  promote)   cmd_promote ;;
  calibrate) cmd_calibrate ;;
  summary)   cmd_summary ;;
  full)      cmd_analyze; echo ""; cmd_promote; echo ""; cmd_calibrate; echo ""; cmd_summary ;;
  *)
    echo "convergio-learnings.sh — Learning loop"
    echo "  summary     Knowledge system status"
    echo "  analyze     Find recurring patterns"
    echo "  promote     Auto-promote to KB + skills"
    echo "  calibrate   Estimation accuracy calibration"
    echo "  full        Run all steps"
    ;;
esac
