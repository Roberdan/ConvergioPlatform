#!/usr/bin/env bash
# mesh-heartbeat.sh — Liveness daemon: writes heartbeat to peer_heartbeats every 30s
# Version: 1.1.0
# Usage: mesh-heartbeat.sh [start|stop|status]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAUDE_HOME="${CLAUDE_HOME:-$HOME/.claude}"
PID_FILE="$CLAUDE_HOME/data/mesh-heartbeat.pid"
INTERVAL=30

# shellcheck source=lib/peers.sh
source "$SCRIPT_DIR/lib/peers.sh"

DAEMON_API="http://localhost:8420"

C='\033[0;36m' G='\033[0;32m' Y='\033[1;33m' R='\033[0;31m' N='\033[0m'
info() { echo -e "${C}[heartbeat]${N} $*"; }
ok() { echo -e "${G}[heartbeat]${N} $*"; }
warn() { echo -e "${Y}[heartbeat]${N} $*" >&2; }
err() { echo -e "${R}[heartbeat]${N} $*" >&2; }

command -v jq &>/dev/null || { err "jq required"; exit 1; }

_load_json() {
	local cpu tasks mem_total=0 mem_used=0
	# uptime load average (1-min) — portable across macOS and Linux
	if command -v uptime &>/dev/null; then
		cpu="$(uptime 2>/dev/null | grep -oE 'load averages?: [0-9]+\.[0-9]+' | grep -oE '[0-9]+\.[0-9]+$' || echo "0")"
	fi
	cpu="${cpu:-0}"

	# count in_progress tasks via daemon API (non-fatal)
	tasks="$(curl -sf "${DAEMON_API}/api/overview" 2>/dev/null | jq -r '.tasks_in_progress // 0' 2>/dev/null || echo "0")"

	# RAM: macOS -> sysctl/vm_stat | Linux -> /proc/meminfo
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

_capabilities() {
	peers_load 2>/dev/null || true
	local self
	self="$(peers_self 2>/dev/null || echo "")"
	if [[ -n "$self" ]]; then
		peers_get "$self" "capabilities" 2>/dev/null || echo ""
	else
		echo ""
	fi
}

_write_heartbeat() {
	local peer_name load_json caps
	peers_load 2>/dev/null || true
	peer_name="$(peers_self 2>/dev/null || echo "$(hostname -s 2>/dev/null || hostname)")"
	# Guard: never write empty peer_name
	if [[ -z "$peer_name" ]]; then
		warn "peer_name is empty — skipping heartbeat write"
		return 1
	fi
	load_json="$(_load_json)"
	caps="$(_capabilities)"

	curl -sf -X POST "${DAEMON_API}/api/heartbeat" \
		-H 'Content-Type: application/json' \
		-d "$(jq -nc --arg pn "$peer_name" --argjson load "$load_json" --arg caps "$caps" \
			'{peer_name:$pn, load_json:$load, capabilities:$caps}')" 2>/dev/null || {
		warn "Heartbeat API write failed (will retry)"
	}
}

# Event emission: check for completed plans/waves/tasks and emit events
_emit_event() {
	local event_type="$1" plan_id="$2" payload="${3:-{}}"
	local peer_name
	peer_name="$(peers_self 2>/dev/null || echo "$(hostname -s 2>/dev/null || hostname)")"
	curl -sf -X POST "${DAEMON_API}/api/coordinator/emit" \
		-H 'Content-Type: application/json' \
		-d "$(jq -nc --arg et "$event_type" --argjson pid "$plan_id" --arg sp "$peer_name" --argjson pl "$payload" \
			'{event_type:$et, plan_id:$pid, source_peer:$sp, payload:$pl}')" 2>/dev/null || true
}

_check_plan_events() {
	local peer_name
	peer_name="$(peers_self 2>/dev/null || echo "$(hostname -s 2>/dev/null || hostname)")"
	local plans_json
	plans_json="$(curl -sf "${DAEMON_API}/api/plan-db/list" 2>/dev/null || echo '[]')"

	echo "$plans_json" | jq -r --arg pn "$peer_name" '
		.[] | select(.status == "doing" and (.execution_host | tostring | contains($pn)))
		| "\(.id)|\(.name)|\(.tasks_done)|\(.tasks_total)"
	' 2>/dev/null | while IFS='|' read -r plan_id plan_name tasks_done tasks_total; do
		[[ -z "$plan_id" ]] && continue
		local pj
		pj="$(cvg plan show "$plan_id" 2>/dev/null || echo '{}')"
		local resolved
		resolved="$(echo "$pj" | jq '[.tasks[]? | select(.status | IN("done","skipped","cancelled"))] | length' 2>/dev/null || echo 0)"
		if [[ "$resolved" -ge "$tasks_total" && "$tasks_total" -gt 0 ]]; then
			_emit_event "plan_completed" "$plan_id" "{\"name\":\"${plan_name}\",\"tasks\":${tasks_total}}"
		fi
		local blocked
		blocked="$(echo "$pj" | jq -r '[.tasks[]? | select(.status == "blocked")] | first | .task_id // empty' 2>/dev/null || echo '')"
		if [[ -n "$blocked" ]]; then
			_emit_event "human_needed" "$plan_id" "{\"action\":\"blocked\",\"task\":\"${blocked}\"}"
		fi
		# Check waves
		echo "$pj" | jq -r '.waves[]? | select(.status == "in_progress") | "\(.wave_id)|\(.name)|\(.id)|\(.tasks_done)|\(.tasks_total)"' 2>/dev/null | while IFS='|' read -r wave_id wave_name _wid wd wt; do
			[[ -z "$wave_id" ]] && continue
			local wave_resolved
			wave_resolved="$(echo "$pj" | jq --argjson wid "$_wid" '[.tasks[]? | select(.wave_id_fk == $wid and (.status | IN("done","skipped","cancelled")))] | length' 2>/dev/null || echo 0)"
			if [[ "$wave_resolved" -ge "$wt" && "$wt" -gt 0 ]]; then
				_emit_event "wave_completed" "$plan_id" "{\"wave\":\"${wave_id}\",\"name\":\"${wave_name}\"}"
			fi
		done
	done
}
_poll_remote_peers() {
	# Coordinator-only: SSH into remote peers, read their heartbeat via API, merge locally
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
			local remote_hb
			remote_hb="$(ssh -o ConnectTimeout=4 -o BatchMode=yes "$alias" \
				"curl -sf http://localhost:8420/api/heartbeat/status 2>/dev/null" 2>/dev/null || echo "")"
			[[ -z "$remote_hb" ]] && return
			# Forward the remote heartbeat to local daemon
			curl -sf -X POST "${DAEMON_API}/api/heartbeat" \
				-H 'Content-Type: application/json' \
				-d "$remote_hb" 2>/dev/null || true
		) &
	done
	# Don't wait — background SSH calls clean up on their own
}

_daemon_loop() {
	local pulse=0
	while true; do
		_write_heartbeat || warn "heartbeat write failed (will retry)"
		# Check for plan/wave/task events (non-fatal)
		_check_plan_events 2>/dev/null || true
		pulse=$((pulse + 1))
		# Every 2 beats (~1 min): poll remote peers (coordinator only)
		if ((pulse % 2 == 0)); then
			_poll_remote_peers 2>/dev/null || true
		fi

		# Every 10 beats (~5 min): cleanup only (NO config sync — git handles that)
		if ((pulse % 10 == 0)); then
			local cleanup_script="$SCRIPT_DIR/mesh-cleanup.sh"
			if [[ -x "$cleanup_script" ]]; then
				"$cleanup_script" --reset-stale --json >>"$CLAUDE_HOME/data/mesh-cleanup.log" 2>&1 &
			fi
		fi
		sleep "$INTERVAL"
	done
}

# Commands

cmd_start() {
	if [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* ]]; then
		info "Windows detected. Use Task Scheduler instead:"
		info "  schtasks /create /tn 'MeshHeartbeat' /tr 'bash -c \"$0 run-once\"' /sc minute /mo 1"
		exit 0
	fi

	if [[ -f "$PID_FILE" ]]; then
		local old_pid
		old_pid="$(cat "$PID_FILE" 2>/dev/null || echo "")"
		if [[ -n "$old_pid" ]] && kill -0 "$old_pid" 2>/dev/null; then
			warn "Already running (PID $old_pid). Use 'stop' first."
			return 1
		else
			warn "Stale PID file found. Removing."
			rm -f "$PID_FILE"
		fi
	fi

	if ! curl -sf "${DAEMON_API}/api/health" >/dev/null 2>&1; then
		err "Daemon not reachable at ${DAEMON_API}"
		err "Start the daemon with: ./daemon/start.sh"
		return 1
	fi

	# Daemonize: run loop in background, disown
	_daemon_loop </dev/null >>"$CLAUDE_HOME/data/mesh-heartbeat.log" 2>&1 &
	local pid=$!
	disown "$pid"
	echo "$pid" >"$PID_FILE"

	ok "Started (PID $pid). Writing heartbeat every ${INTERVAL}s."
	ok "Log: $CLAUDE_HOME/data/mesh-heartbeat.log"

	# Auto-sync disabled: Rust daemon handles CRDT sync on port 9420.
	# Legacy config sync via git bundle was doing destructive DB overwrites.
	info "Rust mesh daemon handles sync (port 9420). Legacy sync disabled."
}

cmd_stop() {
	if [[ ! -f "$PID_FILE" ]]; then
		warn "No PID file found at $PID_FILE. Not running?"
		return 0
	fi
	local pid
	pid="$(cat "$PID_FILE" 2>/dev/null || echo "")"
	if [[ -z "$pid" ]]; then
		warn "Empty PID file. Removing."
		rm -f "$PID_FILE"
		return 0
	fi
	if kill -0 "$pid" 2>/dev/null; then
		kill "$pid" 2>/dev/null && ok "Stopped (PID $pid)." || err "Failed to kill PID $pid"
	else
		warn "Process $pid not running."
	fi
	rm -f "$PID_FILE"
}

cmd_status() {
	# Show daemon status
	local running="no" pid=""
	if [[ -f "$PID_FILE" ]]; then
		pid="$(cat "$PID_FILE" 2>/dev/null || echo "")"
		if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
			running="yes"
		fi
	fi

	if [[ "$running" == "yes" ]]; then
		info "Daemon: RUNNING (PID $pid)"
	else
		info "Daemon: STOPPED"
	fi

	echo ""
	# Show last_seen for all peers via daemon API
	if ! curl -sf "${DAEMON_API}/api/health" >/dev/null 2>&1; then
		warn "Daemon not reachable at ${DAEMON_API}"
		return 0
	fi

	printf "  %-20s %-22s %-30s %s\n" "PEER" "LAST_SEEN" "LOAD" "CAPABILITIES"
	printf "  %-20s %-22s %-30s %s\n" "----" "---------" "----" "------------"

	local now
	now="$(date +%s)"

	local hb_status
	hb_status="$(curl -sf "${DAEMON_API}/api/heartbeat/status" 2>/dev/null || echo '[]')"
	echo "$hb_status" | jq -r '.[] | "\(.peer_name)|\(.last_seen)|\(.load_json // "{}")|\(.capabilities // "")"' 2>/dev/null | while IFS='|' read -r peer_name last_seen load_json caps; do
		[[ -z "$peer_name" ]] && continue
		local age_str="never"
		if [[ -n "$last_seen" && "$last_seen" =~ ^[0-9]+$ ]]; then
			local age=$((now - last_seen))
			if ((age < 60)); then
				age_str="${age}s ago"
			elif ((age < 3600)); then
				age_str="$((age / 60))m ago"
			else
				age_str="$((age / 3600))h ago"
			fi
		fi
		printf "  %-20s %-22s %-30s %s\n" \
			"$peer_name" "$age_str" "${load_json:-{}}" "${caps:-}"
	done

	echo ""
}

cmd_daemon() {
	# Foreground mode for service managers (launchd, systemd).
	# Runs the heartbeat loop WITHOUT forking — the OS manages the process.
	if ! curl -sf "${DAEMON_API}/api/health" >/dev/null 2>&1; then
		err "Daemon not reachable at ${DAEMON_API}"
		return 1
	fi
	info "Running in foreground (service mode). PID=$$"
	echo "$$" >"$PID_FILE"
	trap 'rm -f "$PID_FILE"; info "Daemon stopped."; exit 0' SIGTERM SIGINT
	_daemon_loop
}

# Main

case "${1:-}" in
start) cmd_start ;;
stop) cmd_stop ;;
status) cmd_status ;;
daemon) cmd_daemon ;;
ping) _write_heartbeat && ok "Heartbeat written" ;;
-h | --help | help)
	echo "Usage: $(basename "$0") [start|stop|status|daemon|ping]"
	echo "  start  — start heartbeat daemon (every ${INTERVAL}s)"
	echo "  stop   — stop heartbeat daemon"
	echo "  status — show last_seen for all peers"
	echo "  daemon — foreground mode for launchd/systemd"
	echo "  ping   — write a single heartbeat now"
	;;
"")
	err "No command given. Use: start|stop|status|daemon"
	echo "Usage: $(basename "$0") [start|stop|status|daemon]" >&2
	exit 1
	;;
*)
	err "Unknown command: $1. Use: start|stop|status|daemon"
	exit 1
	;;
esac
