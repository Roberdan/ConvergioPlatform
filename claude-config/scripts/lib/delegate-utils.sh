#!/usr/bin/env bash
# delegate-utils.sh — Utility functions for worker scripts
# Version: 2.0.0 — migrated from sqlite3 to cvg CLI / daemon API
command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

_DU_SCRIPTS="${SCRIPT_DIR:-$HOME/.claude/scripts}"
DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"

safe_update_task() {
    local task_id="$1" status="$2"
    shift 2
    local notes="" output_data=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --tokens) shift 2 ;;
            --output-data) output_data="$2"; shift 2 ;;
            *) notes="${notes:+$notes }$1"; shift ;;
        esac
    done
    if [[ "$status" == "done" ]]; then
        "${_DU_SCRIPTS}/plan-db-safe.sh" update-task "$task_id" done "$notes" \
            ${output_data:+--output-data "$output_data"} 2>/dev/null || \
        cvg task update "$task_id" done "${notes:-Completed}" 2>/dev/null || \
        curl -sf -X POST "${DAEMON_URL}/api/plan-db/task/update" \
            -H 'Content-Type: application/json' \
            -d "{\"task_id\":${task_id},\"status\":\"done\",\"notes\":$(jq -n --arg n "$notes" '$n')}" >/dev/null 2>&1 || true
    else
        cvg task update "$task_id" "$status" "${notes:-}" 2>/dev/null || \
        curl -sf -X POST "${DAEMON_URL}/api/plan-db/task/update" \
            -H 'Content-Type: application/json' \
            -d "{\"task_id\":${task_id},\"status\":\"${status}\",\"notes\":$(jq -n --arg n "$notes" '$n')}" >/dev/null 2>&1 || true
    fi
}

verify_work_done() {
    local wt="${1:-.}"
    [[ -d "$wt" ]] || return 1
    local changes
    changes="$(git -C "$wt" status --porcelain 2>/dev/null)"
    [[ -n "$changes" ]]
}

log_delegation() {
    local task_id="${1:-}" plan_id="${2:-}" project_id="${3:-}" agent="${4:-}" model="${5:-}"
    local prompt_tokens="${6:-0}" output_tokens="${7:-0}" duration_ms="${8:-0}" exit_code="${9:-0}"
    local thor_result="${10:-UNKNOWN}" retry="${11:-0}" status="${12:-unknown}"
    # Write delegation log via daemon API
    # TODO: needs daemon endpoint for delegation_log insert
    curl -sf -X POST "${DAEMON_URL}/api/plan-db/delegation-log" \
        -H 'Content-Type: application/json' \
        -d "{\"task_id\":\"${task_id}\",\"plan_id\":${plan_id},\"project_id\":\"${project_id}\",\"agent\":\"${agent}\",\"model\":\"${model}\",\"prompt_tokens\":${prompt_tokens},\"output_tokens\":${output_tokens},\"duration_ms\":${duration_ms},\"exit_code\":${exit_code},\"thor_result\":\"${thor_result}\",\"retry_count\":${retry},\"status\":\"${status}\"}" >/dev/null 2>&1 || true
}
