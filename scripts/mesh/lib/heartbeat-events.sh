#!/usr/bin/env bash
# heartbeat-events.sh — Event emission, load metrics, and peer polling for mesh-heartbeat
# Sourced by mesh-heartbeat.sh. Requires: _db(), peers_* from lib/peers.sh
# Version: 1.0.0

# Collect system load metrics as JSON (portable: macOS + Linux)
_load_json() {
	local cpu tasks mem_total=0 mem_used=0
	if command -v uptime &>/dev/null; then
		cpu="$(uptime 2>/dev/null | grep -oE 'load averages?: [0-9]+\.[0-9]+' | grep -oE '[0-9]+\.[0-9]+$' || echo "0")"
	fi
	cpu="${cpu:-0}"
	tasks="$(_db "SELECT COUNT(*) FROM tasks WHERE status='in_progress';" 2>/dev/null || echo "0")"
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

# Emit a mesh event (idempotent: skips if pending event already exists)
_emit_event() {
	local event_type="$1" plan_id="$2" payload="${3:-}"
	local peer_name
	peer_name="$(peers_self 2>/dev/null || echo "$(hostname -s 2>/dev/null || hostname)")"
	local exists
	exists=$(_db "SELECT COUNT(*) FROM mesh_events WHERE event_type='${event_type}' AND plan_id=${plan_id} AND source_peer='${peer_name}' AND status='pending';" 2>/dev/null || echo "0")
	if [[ "$exists" -eq 0 ]]; then
		_db "INSERT INTO mesh_events (event_type, plan_id, source_peer, payload) VALUES ('${event_type}', ${plan_id}, '${peer_name}', '${payload}');" 2>/dev/null || true
	fi
}

# Check active plans/waves for completion or blockage and emit events
_check_plan_events() {
	local peer_name
	peer_name="$(peers_self 2>/dev/null || echo "$(hostname -s 2>/dev/null || hostname)")"
	while IFS='|' read -r plan_id plan_name tasks_done tasks_total; do
		[[ -z "$plan_id" ]] && continue
		local resolved
		resolved=$(_db "SELECT COUNT(*) FROM tasks t JOIN waves w ON t.wave_id_fk=w.id WHERE w.plan_id=${plan_id} AND t.status IN ('done','skipped','cancelled');" 2>/dev/null || echo "0")
		if [[ "$resolved" -ge "$tasks_total" && "$tasks_total" -gt 0 ]]; then
			_emit_event "plan_completed" "$plan_id" "{\"name\":\"${plan_name}\",\"tasks\":${tasks_total}}"
		fi
		local blocked
		blocked=$(_db "SELECT task_id FROM tasks t JOIN waves w ON t.wave_id_fk=w.id WHERE w.plan_id=${plan_id} AND t.status='blocked' LIMIT 1;" 2>/dev/null || echo "")
		if [[ -n "$blocked" ]]; then
			_emit_event "human_needed" "$plan_id" "{\"action\":\"blocked\",\"task\":\"${blocked}\"}"
		fi
	done < <(_db "SELECT id, name, tasks_done, tasks_total FROM plans WHERE status='doing' AND (execution_host LIKE '%${peer_name}%' OR execution_host='${peer_name}');" 2>/dev/null || true)
	while IFS='|' read -r wave_id wave_name plan_id wd wt; do
		[[ -z "$wave_id" ]] && continue
		local wave_resolved
		wave_resolved=$(_db "SELECT COUNT(*) FROM tasks WHERE wave_id_fk=(SELECT id FROM waves WHERE wave_id='${wave_id}' AND plan_id=${plan_id}) AND status IN ('done','skipped','cancelled');" 2>/dev/null || echo "0")
		if [[ "$wave_resolved" -ge "$wt" && "$wt" -gt 0 ]]; then
			_emit_event "wave_completed" "$plan_id" "{\"wave\":\"${wave_id}\",\"name\":\"${wave_name}\"}"
		fi
	done < <(_db "SELECT w.wave_id, w.name, w.plan_id, w.tasks_done, w.tasks_total FROM waves w JOIN plans p ON w.plan_id=p.id WHERE p.status='doing' AND w.status='in_progress' AND (p.execution_host LIKE '%${peer_name}%' OR p.execution_host='${peer_name}');" 2>/dev/null || true)
}

# Coordinator-only: SSH into remote peers, read their heartbeat, merge locally
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
		# Async SSH: read remote heartbeat (timeout 5s, non-blocking)
		(
			local row
			row="$(ssh -o ConnectTimeout=4 -o BatchMode=yes "$alias" \
				"sqlite3 ~/.claude/data/dashboard.db \"SELECT peer_name, last_seen, load_json, capabilities FROM peer_heartbeats WHERE peer_name='$name' LIMIT 1;\"" 2>/dev/null || echo "")"
			[[ -z "$row" ]] && return
			local rp rl rj rc
			IFS='|' read -r rp rl rj rc <<< "$row"
			[[ -z "$rp" || -z "$rl" ]] && return
			rj="${rj//\'/\'\'}"
			rc="${rc//\'/\'\'}"
			_db "INSERT OR REPLACE INTO peer_heartbeats (peer_name, last_seen, load_json, capabilities) VALUES ('${rp}', ${rl}, '${rj}', '${rc}');" 2>/dev/null || true
		) &
	done
	# Don't wait — background SSH calls clean up on their own
}
