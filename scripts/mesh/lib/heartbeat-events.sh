#!/usr/bin/env bash
# heartbeat-events.sh — Event emission, load metrics, and peer polling for mesh-heartbeat
# Sourced by mesh-heartbeat.sh. Requires: peers_* from lib/peers.sh, DAEMON_URL
# Version: 2.0.0

# Collect system load metrics as JSON (portable: macOS + Linux)
_load_json() {
	local cpu tasks mem_total=0 mem_used=0
	if command -v uptime &>/dev/null; then
		cpu="$(uptime 2>/dev/null | grep -oE 'load averages?: [0-9]+\.[0-9]+' | grep -oE '[0-9]+\.[0-9]+$' || echo "0")"
	fi
	cpu="${cpu:-0}"
	# Get in-progress task count from daemon API
	tasks=$(curl -sf "${DAEMON_URL}/api/tasks/distribution" 2>/dev/null | jq -r '.in_progress // 0' 2>/dev/null || echo "0")
	if [[ "$(uname)" == "Darwin" ]]; then
		mem_total=$(sysctl -n hw.memsize 2>/dev/null | awk '{printf "%.1f", $1/1073741824}')
		local pages_free pages_inactive page_size used_bytes
		pages_free=$(vm_stat 2>/dev/null | awk '/Pages free/ {gsub(/\./,"",$3); print $3}')
		pages_inactive=$(vm_stat 2>/dev/null | awk '/Pages inactive/ {gsub(/\./,"",$3); print $3}')
		page_size=$(sysctl -n hw.pagesize 2>/dev/null || echo 16384)
		used_bytes=$(((${mem_total%.*} * 1073741824) - (${pages_free:-0} + ${pages_inactive:-0}) * page_size))
		mem_used=$(echo "$used_bytes" | awk '{printf "%.1f", $1/1073741824}')
	elif [[ -f /proc/meminfo ]]; then
		mem_total=$(awk '/MemTotal/ {printf "%.1f", $2/1048576}' /proc/meminfo 2>/dev/null)
		local mem_avail
		mem_avail=$(awk '/MemAvailable/ {printf "%.1f", $2/1048576}' /proc/meminfo 2>/dev/null)
		mem_used=$(echo "$mem_total $mem_avail" | awk '{printf "%.1f", $1-$2}')
	fi
	printf '{"cpu":%s,"tasks":%s,"mem_used_gb":%s,"mem_total_gb":%s}' "$cpu" "$tasks" "${mem_used:-0}" "${mem_total:-0}"
}

# Emit a mesh event via daemon API
_emit_event() {
	local event_type="$1" plan_id="$2" payload="${3:-}"
	local peer_name
	peer_name="$(peers_self 2>/dev/null || echo "$(hostname -s 2>/dev/null || hostname)")"

	curl -sf -X POST "${DAEMON_URL}/api/events" \
		-H 'Content-Type: application/json' \
		-d "{\"event_type\":\"${event_type}\",\"plan_id\":${plan_id},\"source_peer\":\"${peer_name}\",\"payload\":\"${payload}\"}" 2>/dev/null || true
}

# Check active plans/waves for completion or blockage and emit events
_check_plan_events() {
	local peer_name
	peer_name="$(peers_self 2>/dev/null || echo "$(hostname -s 2>/dev/null || hostname)")"

	# Get active plans from daemon API
	local plans_json
	plans_json=$(curl -sf "${DAEMON_URL}/api/overview" 2>/dev/null || echo "")
	[[ -z "$plans_json" ]] && return 0

	echo "$plans_json" | jq -r '.active_plans[]? | [(.id // ""), (.name // ""), (.tasks_done // 0), (.tasks_total // 0)] | @tsv' 2>/dev/null | while IFS=$'\t' read -r plan_id plan_name tasks_done tasks_total; do
		[[ -z "$plan_id" ]] && continue

		# Check if all tasks resolved
		if [[ "$tasks_done" -ge "$tasks_total" && "$tasks_total" -gt 0 ]]; then
			_emit_event "plan_completed" "$plan_id" "{\"name\":\"${plan_name}\",\"tasks\":${tasks_total}}"
		fi

		# Check for blocked tasks
		local blocked_json
		blocked_json=$(curl -sf "${DAEMON_URL}/api/tasks/blocked" 2>/dev/null || echo "")
		if [[ -n "$blocked_json" ]]; then
			local blocked_task
			blocked_task=$(echo "$blocked_json" | jq -r "[.tasks[]? | select(.plan_id == ${plan_id})] | first | .task_id // empty" 2>/dev/null || echo "")
			if [[ -n "$blocked_task" ]]; then
				_emit_event "human_needed" "$plan_id" "{\"action\":\"blocked\",\"task\":\"${blocked_task}\"}"
			fi
		fi
	done

	# Check waves via plan data
	# Wave completion detection uses the overview data already fetched
	# Detailed wave checks happen through the autopilot, not heartbeat
}

# Coordinator-only: poll remote peers via daemon mesh API
_poll_remote_peers() {
	peers_load 2>/dev/null || return 0
	local self
	self="$(peers_self 2>/dev/null || echo "")"
	local my_role
	my_role="$(peers_get "$self" "role" 2>/dev/null || echo "worker")"
	[[ "$my_role" != "coordinator" ]] && return 0

	for name in $_PEERS_ACTIVE; do
		[[ "$name" == "$self" ]] && continue
		local alias
		alias="$(_peers_get_raw "$name" "ssh_alias")"
		[[ -z "$alias" ]] && continue
		# Async SSH: read remote heartbeat via their daemon API (timeout 5s, non-blocking)
		(
			local row
			row="$(ssh -o ConnectTimeout=4 -o BatchMode=yes "$alias" \
				"curl -sf http://localhost:8420/api/heartbeat/status 2>/dev/null | jq -r '.peers[]? | select(.peer_name==\"$name\") | [.peer_name, (.last_seen // \"\"), (.load_json // \"{}\"), (.capabilities // \"\")] | @tsv'" 2>/dev/null || echo "")"
			[[ -z "$row" ]] && return
			local rp rl rj rc
			IFS=$'\t' read -r rp rl rj rc <<< "$row"
			[[ -z "$rp" || -z "$rl" ]] && return
			# Write remote heartbeat to local daemon
			curl -sf -X POST "${DAEMON_URL}/api/heartbeat" \
				-H 'Content-Type: application/json' \
				-d "{\"peer\":\"${rp}\",\"status\":\"online\",\"last_seen\":${rl},\"load_json\":${rj},\"capabilities\":\"${rc}\"}" 2>/dev/null || true
		) &
	done
	# Don't wait — background SSH calls clean up on their own
}
