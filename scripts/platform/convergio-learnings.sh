#!/usr/bin/env bash
# convergio-learnings.sh — Learning loop: detect patterns, promote to knowledge/skills
# Analyzes plan_learnings for recurring patterns and auto-promotes
set -uo pipefail

PLATFORM_DIR="${CONVERGIO_PLATFORM_DIR:-$HOME/GitHub/ConvergioPlatform}"
DB="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"

_db() { sqlite3 "$DB" "$1" 2>/dev/null; }

cmd_analyze() {
  echo "=== Learning Pattern Analysis ==="
  echo ""

  echo "--- Recurring learning categories ---"
  _db "SELECT category, severity, count(*) as occurrences
       FROM plan_learnings
       GROUP BY category, severity
       HAVING occurrences >= 2
       ORDER BY occurrences DESC
       LIMIT 20;" | while IFS='|' read -r cat sev count; do
    printf "  %-20s %-10s %s occurrences\n" "$cat" "$sev" "$count"
  done

  echo ""
  echo "--- Most common learning titles ---"
  _db "SELECT title, count(*) as freq
       FROM plan_learnings
       GROUP BY title
       HAVING freq >= 2
       ORDER BY freq DESC
       LIMIT 10;" | while IFS='|' read -r title freq; do
    printf "  [%sx] %s\n" "$freq" "$title"
  done

  echo ""
  echo "--- Actionable learnings not yet acted on ---"
  _db "SELECT plan_id, title FROM plan_learnings
       WHERE actionable = 1 AND (action_taken IS NULL OR action_taken = '')
       ORDER BY created_at DESC
       LIMIT 10;" | while IFS='|' read -r pid title; do
    echo "  Plan $pid: $title"
  done
}

cmd_promote() {
  echo "=== Auto-Promoting Recurring Learnings ==="
  local promoted=0

  # Find learning titles that appear 3+ times → promote to knowledge_base
  _db "SELECT title, category, count(*) as freq, group_concat(DISTINCT plan_id) as plans
       FROM plan_learnings
       GROUP BY title
       HAVING freq >= 3
       ORDER BY freq DESC;" | while IFS='|' read -r title cat freq plans; do

    # Check if already in knowledge_base
    local existing
    existing=$(_db "SELECT count(*) FROM knowledge_base WHERE title = '$(echo "$title" | sed "s/'/''/g")';")

    if [ "${existing:-0}" -eq 0 ]; then
      _db "INSERT INTO knowledge_base (domain, title, content, confidence, source_type, source_ref)
           VALUES ('$cat', '$(echo "$title" | sed "s/'/''/g")', 'Auto-promoted from $freq plan learnings',
                   $(echo "scale=2; 0.5 + ($freq * 0.1)" | bc), 'learned', 'Plans: $plans');"
      echo "  PROMOTED: $title (${freq}x across plans $plans)"
      promoted=$((promoted + 1))
    fi
  done

  # Find knowledge_base entries with hit_count >= 5 → promote to earned_skills
  _db "SELECT id, domain, title, content, hit_count
       FROM knowledge_base
       WHERE promoted = 0 AND hit_count >= 5
       ORDER BY hit_count DESC;" | while IFS='|' read -r id domain title content hits; do

    local skill_name
    skill_name=$(echo "$title" | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | tr -cd 'a-z0-9-' | cut -c1-50)

    _db "INSERT OR IGNORE INTO earned_skills (name, domain, content, confidence, hit_count, source)
         VALUES ('$skill_name', '$domain', '$(echo "$content" | sed "s/'/''/g")', 'medium', $hits, 'auto-promoted');"
    _db "UPDATE knowledge_base SET promoted = 1, skill_name = '$skill_name' WHERE id = $id;"
    echo "  SKILL: $skill_name (from KB entry with $hits hits)"
  done

  echo "  Promoted: $promoted learnings to knowledge base"
}

cmd_calibrate() {
  echo "=== Estimation Calibration ==="
  bash "$PLATFORM_DIR/claude-config/scripts/plan-db.sh" calibrate-estimates 2>/dev/null || {
    echo "  No calibration data yet"
  }
}

cmd_summary() {
  echo "=== Knowledge System Status ==="
  echo "  Knowledge base: $(_db "SELECT count(*) FROM knowledge_base;") entries"
  echo "  Plan learnings: $(_db "SELECT count(*) FROM plan_learnings;") entries"
  echo "  Agent skills:   $(_db "SELECT count(*) FROM agent_skills;") mappings"
  echo "  Earned skills:  $(_db "SELECT count(*) FROM earned_skills;" 2>/dev/null || echo "0") skills"
  echo "  Agent catalog:  $(_db "SELECT count(*) FROM agent_catalog;") agents"
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
