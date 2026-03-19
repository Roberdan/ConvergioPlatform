#!/usr/bin/env bash
# pianits.sh — CLI plan status dashboard (terminal version of piani)
# Usage: pianits [project_id] | pianits kanban | pianits tree <plan_id>
set -euo pipefail

PLAN_DB="plan-db.sh"
command -v "$PLAN_DB" >/dev/null 2>&1 || { echo "plan-db.sh not in PATH"; exit 1; }

B='\033[1m' G='\033[0;32m' C='\033[0;36m' Y='\033[1;33m' N='\033[0m'

CMD="${1:-overview}"
shift 2>/dev/null || true

case "$CMD" in
  kanban|k)
    "$PLAN_DB" kanban "$@"
    ;;
  tree|t)
    "$PLAN_DB" execution-tree "$@"
    ;;
  json|j)
    "$PLAN_DB" json "$@"
    ;;
  agents|a)
    "$PLAN_DB" agent-status "$@"
    ;;
  tokens)
    "$PLAN_DB" agent-tokens "$@"
    ;;
  overview|o|"")
    echo ""
    echo -e "${B}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
    echo -e "${B}  Convergio Plan Status${N}"
    echo -e "${B}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
    echo ""
    "$PLAN_DB" status "$@"
    echo ""
    echo -e "${C}Commands: kanban | tree <id> | json <id> | agents | tokens${N}"
    echo ""
    ;;
  *)
    # Pass through to plan-db.sh
    "$PLAN_DB" status "$CMD" "$@"
    ;;
esac
