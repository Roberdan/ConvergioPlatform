#!/usr/bin/env bash
set -euo pipefail
# DB Digest - Compact dashboard DB summaries for plans/tasks/waves
# Usage: db-digest.sh <plans|tasks|waves|stats> [plan_id] [--no-cache] [--compact]
# Version: 1.1.0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/digest-cache.sh"
source "$SCRIPT_DIR/lib/cost-calculator.sh"

DAEMON_API="http://localhost:8420"
CACHE_TTL=10
NO_CACHE=0
COMPACT=0

command -v jq &>/dev/null || { echo '{"status":"error","msg":"jq required"}' >&2; exit 1; }

digest_check_compact "$@"

require_daemon() {
	curl -sf "${DAEMON_API}/api/health" >/dev/null 2>&1 || {
		jq -n '{status:"error",msg:"daemon not reachable at '"$DAEMON_API"'"}' >&2
		exit 1
	}
}

cmd_plans() {
	local all_plans
	all_plans="$(curl -sf "${DAEMON_API}/api/plan-db/list" 2>/dev/null || echo '[]')"
	echo "$all_plans" | jq '[.[] | select(.status == "todo" or .status == "doing") |
		{id, project_id, name, status, tasks_done, tasks_total,
		 progress_pct: (if .tasks_total > 0 then ((.tasks_done * 100.0 / .tasks_total) * 10 | round / 10) else 0 end)}
	] | sort_by(if .status == "doing" then 0 elif .status == "todo" then 1 else 2 end)'
}

cmd_tasks() {
	local plan_id="${1:-}"
	[[ "$plan_id" =~ ^[0-9]+$ ]] || {
		echo '{"status":"error","msg":"plan_id required for tasks"}' >&2
		exit 1
	}
	local plan_json
	plan_json="$(cvg plan show "$plan_id" 2>/dev/null || echo '{}')"
	echo "$plan_json" | jq --argjson pid "$plan_id" '{
		plan_id: $pid,
		total_tasks: ([.tasks[]] | length),
		pending: ([.tasks[] | select(.status == "pending")] | length),
		in_progress: ([.tasks[] | select(.status == "in_progress")] | length),
		submitted: ([.tasks[] | select(.status == "submitted")] | length),
		done: ([.tasks[] | select(.status == "done")] | length),
		blocked: ([.tasks[] | select(.status == "blocked")] | length),
		cancelled: ([.tasks[] | select(.status == "cancelled")] | length),
		skipped: ([.tasks[] | select(.status == "skipped")] | length)
	}'
}

cmd_waves() {
	local plan_id="${1:-}"
	[[ "$plan_id" =~ ^[0-9]+$ ]] || {
		echo '{"status":"error","msg":"plan_id required for waves"}' >&2
		exit 1
	}
	local plan_json
	plan_json="$(cvg plan show "$plan_id" 2>/dev/null || echo '{}')"
	echo "$plan_json" | jq '[.waves[]? |
		{id, wave_id, name, status, tasks_done, tasks_total,
		 progress_pct: (if .tasks_total > 0 then ((.tasks_done * 100.0 / .tasks_total) * 10 | round / 10) else 0 end),
		 merge_mode, theme}
	] | sort_by(.position // 0, .id)'
}

cmd_stats() {
	local overview
	overview="$(curl -sf "${DAEMON_API}/api/overview" 2>/dev/null || echo '{}')"
	echo "$overview" | jq '{
		total_plans: (.total_plans // 0),
		done: (.plans_done // 0),
		active: (.plans_active // 0),
		cancelled: (.plans_cancelled // 0)
	}'
}

cmd_token_stats() {
	local metrics
	metrics="$(curl -sf "${DAEMON_API}/api/metrics/summary" 2>/dev/null || echo '{}')"
	echo "$metrics" | jq '{
		tasks_done: (.tasks_done // 0),
		tasks_tracked: (.tasks_tracked // 0),
		total_tokens: (.total_tokens // 0),
		avg_tokens_per_task: (.avg_tokens_per_task // 0),
		copilot_tasks: (.copilot_tasks // 0),
		claude_tasks: (.claude_tasks // 0)
	}'
}

cmd_cost_report() {
	[[ "${1:-}" =~ ^[0-9]+$ ]] || {
		echo '{"status":"error","msg":"plan_id required"}' >&2
		exit 1
	}
	calc_cost_from_token_usage "$1"
}

cmd_monthly() {
	local overview
	overview="$(curl -sf "${DAEMON_API}/api/overview" 2>/dev/null || echo '{}')"
	echo "$overview" | jq '.monthly // []'
}

print_help() {
	echo "Usage: db-digest.sh <plans|tasks|waves|stats|token-stats|monthly|cost-report> [plan_id] [--no-cache] [--compact]"
}

COMMAND=""
PLAN_ID=""
for arg in "$@"; do
	case "$arg" in
	--no-cache)
		NO_CACHE=1
		;;
	--compact) ;;
	--help | -h | help)
		print_help
		exit 0
		;;
	plans | tasks | waves | stats | token-stats | monthly | cost-report)
		[[ -z "$COMMAND" ]] && COMMAND="$arg"
		;;
	*)
		[[ -z "$PLAN_ID" ]] && PLAN_ID="$arg"
		;;
	esac
done

[[ -n "$COMMAND" ]] || {
	print_help
	exit 0
}

require_daemon
CACHE_KEY="db-${COMMAND}-${PLAN_ID:-none}"
if [[ "$NO_CACHE" -eq 0 ]] && digest_cache_get "$CACHE_KEY" "$CACHE_TTL"; then
	exit 0
fi

case "$COMMAND" in
plans)
	RESULT=$(cmd_plans)
	FILTER='map({id, status, progress_pct})'
	;;
tasks)
	RESULT=$(cmd_tasks "$PLAN_ID")
	FILTER='{plan_id, total_tasks, in_progress, submitted, done, blocked}'
	;;
waves)
	RESULT=$(cmd_waves "$PLAN_ID")
	FILTER='map({id, wave_id, status, progress_pct, merge_mode})'
	;;
stats)
	RESULT=$(cmd_stats)
	FILTER='{total_plans, active, done, cancelled}'
	;;
token-stats)
	RESULT=$(cmd_token_stats)
	FILTER='{tasks_done, tasks_tracked, total_tokens, avg_tokens_per_task, copilot_tasks, claude_tasks}'
	;;
monthly)
	RESULT=$(cmd_monthly)
	FILTER='map({month, plans, tasks_done})'
	;;
cost-report)
	cmd_cost_report "$PLAN_ID"
	exit 0
	;;
*)
	print_help
	exit 1
	;;
esac

echo "$RESULT" | digest_cache_set "$CACHE_KEY"
echo "$RESULT" | COMPACT=$COMPACT digest_compact_filter "$FILTER"
