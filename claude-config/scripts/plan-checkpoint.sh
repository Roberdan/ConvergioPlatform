#!/bin/bash
# plan-checkpoint.sh — Save/restore lean plan checkpoints for context continuity
# Called by: preserve-context.sh (PreCompact hook), coordinator manually
# Version: 2.0.0 — No sqlite3, lean output, no MEMORY.md mutation
set -euo pipefail

CHECKPOINT_DIR="$HOME/.claude/data/checkpoints"
API_BASE="http://localhost:8420/api"

usage() {
	cat <<'EOF'
Usage: plan-checkpoint.sh <command> [args]
  save <plan_id>         Save lean plan checkpoint (max 10 lines)
  restore <plan_id>      Print checkpoint for context injection
  save-auto              Auto-detect active plan via daemon API
  status                 List all active checkpoints
EOF
	exit 1
}

# Fetch plan JSON from cvg CLI (wraps daemon API); fail gracefully
fetch_plan() {
	local plan_id="$1"
	local raw
	raw=$(cvg plan show "$plan_id" 2>/dev/null | sed '1{/^[0-9]\{4\}-/d;}') || echo ""
	echo "$raw"
}

save_checkpoint() {
	local plan_id="$1"
	mkdir -p "$CHECKPOINT_DIR"

	local plan_json
	plan_json=$(fetch_plan "$plan_id")

	if [[ -z "$plan_json" || "$plan_json" == "null" ]]; then
		echo "ERROR: Cannot reach daemon or plan $plan_id not found" >&2
		return 1
	fi

	# Extract fields via python (available on macOS)
	local summary
	summary=$(/usr/bin/python3 -c "
import json, sys
try:
    d = json.loads(sys.argv[1])
    plan = d.get('plan', d) if isinstance(d, dict) else {}
    name = plan.get('name', '?')
    status = plan.get('status', '?')
    waves = d.get('waves', plan.get('waves', []))
    current_wave = ''
    for w in waves:
        if w.get('status') == 'in_progress':
            current_wave = w.get('wave_id', w.get('id', ''))
            break
    tasks = d.get('tasks', plan.get('tasks', []))
    counts = {}
    for t in tasks:
        s = t.get('status', 'unknown')
        counts[s] = counts.get(s, 0) + 1
    total = sum(counts.values())
    done = counts.get('done', 0)
    parts = [f'{v} {k}' for k, v in sorted(counts.items())]
    task_summary = ', '.join(parts) if parts else '0 tasks'
    print(f'{name}')
    print(f'{status}')
    print(f'{current_wave}')
    print(f'{done}/{total} done')
    print(f'{task_summary}')
except Exception as e:
    print(f'?\\n?\\n?\\n0/0\\nerror: {e}', file=sys.stderr)
    sys.exit(1)
" "$plan_json" 2>/dev/null) || {
		echo "ERROR: Failed to parse plan $plan_id JSON" >&2
		return 1
	}

	local plan_name plan_status current_wave progress task_summary
	plan_name=$(echo "$summary" | sed -n '1p')
	plan_status=$(echo "$summary" | sed -n '2p')
	current_wave=$(echo "$summary" | sed -n '3p')
	progress=$(echo "$summary" | sed -n '4p')
	task_summary=$(echo "$summary" | sed -n '5p')

	local checkpoint_file="$CHECKPOINT_DIR/plan-${plan_id}.md"
	{
		echo "# Checkpoint: Plan $plan_id"
		echo "**${plan_name}** | Status: ${plan_status} | Wave: ${current_wave:-none}"
		echo "Tasks: ${progress} (${task_summary})"
		echo "Recovery: \`cvg plan execution-tree ${plan_id}\`"
	} >"$checkpoint_file"

	echo "$checkpoint_file"
}

save_auto() {
	local plans_json
	plans_json=$(curl -s --max-time 3 "${API_BASE}/plan-db/plans" 2>/dev/null || echo "")

	if [[ -z "$plans_json" ]]; then
		echo "No daemon response — skipping checkpoint" >&2
		exit 0
	fi

	local plan_id
	plan_id=$(/usr/bin/python3 -c "
import json, sys
try:
    plans = json.loads(sys.argv[1])
    if isinstance(plans, dict):
        plans = plans.get('plans', plans.get('data', []))
    for p in reversed(plans if isinstance(plans, list) else []):
        if p.get('status') == 'doing':
            print(p.get('id', '')); sys.exit(0)
except Exception:
    pass
" "$plans_json" 2>/dev/null) || plan_id=""

	if [[ -z "$plan_id" ]]; then
		echo "No active plan found" >&2
		exit 0
	fi

	save_checkpoint "$plan_id"
}

restore_checkpoint() {
	local plan_id="$1"
	local checkpoint_file="$CHECKPOINT_DIR/plan-${plan_id}.md"

	if [[ -f "$checkpoint_file" ]]; then
		cat "$checkpoint_file"
	else
		save_checkpoint "$plan_id" >/dev/null 2>&1 || true
		[[ -f "$checkpoint_file" ]] && cat "$checkpoint_file"
	fi
}

show_status() {
	if [[ ! -d "$CHECKPOINT_DIR" ]]; then
		echo "No checkpoints found"
		return 0
	fi
	local found=false
	for f in "$CHECKPOINT_DIR"/plan-*.md; do
		[[ -f "$f" ]] || continue
		found=true
		local pid
		pid=$(basename "$f" .md | sed 's/plan-//')
		local updated
		updated=$(stat -f '%Sm' -t '%Y-%m-%d %H:%M' "$f" 2>/dev/null || echo "?")
		echo "Plan $pid | Updated: $updated"
	done
	if [[ "$found" == false ]]; then
		echo "No checkpoints found"
	fi
}

case "${1:-}" in
save) save_checkpoint "${2:?plan_id required}" ;;
restore) restore_checkpoint "${2:?plan_id required}" ;;
save-auto) save_auto ;;
status) show_status ;;
*) usage ;;
esac
