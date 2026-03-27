#!/usr/bin/env bash
# mesh-heartbeat.sh — Liveness daemon: writes heartbeat via daemon API every 30s
# Version: 2.0.0
# Usage: mesh-heartbeat.sh [start|stop|status]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAUDE_HOME="${CLAUDE_HOME:-$HOME/.claude}"
PID_FILE="$CLAUDE_HOME/data/mesh-heartbeat.pid"
INTERVAL=30
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }
command -v curl &>/dev/null || { echo "ERROR: curl required" >&2; exit 1; }

# shellcheck source=lib/peers.sh
source "$SCRIPT_DIR/lib/peers.sh"
# shellcheck source=lib/heartbeat-events.sh
source "$SCRIPT_DIR/lib/heartbeat-events.sh"

C='\033[0;36m' G='\033[0;32m' Y='\033[1;33m' R='\033[0;31m' N='\033[0m'
info() { echo -e "${C}[heartbeat]${N} $*"; }
ok() { echo -e "${G}[heartbeat]${N} $*"; }
warn() { echo -e "${Y}[heartbeat]${N} $*" >&2; }
err() { echo -e "${R}[heartbeat]${N} $*" >&2; }

# Helpers

_daemon_up() { curl -sf --connect-timeout 2 "${DAEMON_URL}/api/health" > /dev/null 2>&1; }

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

	curl -sf -X POST "${DAEMON_URL}/api/heartbeat" \
		-H 'Content-Type: application/json' \
		-d "{\"peer\":\"${peer_name}\",\"status\":\"online\",\"load_json\":${load_json},\"capabilities\":\"${caps}\"}" 2>/dev/null || {
		warn "Heartbeat API write failed (will retry)"
	}
}

_run_offline_recovery() {
	# On reconnect after offline period, apply any pending patches
	local recovery_script="$SCRIPT_DIR/offline-recovery.sh"
	[[ -x "$recovery_script" ]] || return 0
	"$recovery_script" 2>>"$CLAUDE_HOME/data/offline-recovery.log" &
}

_daemon_loop() {
	local pulse=0
	# Track whether previous beat was reachable (for reconnect detection)
	local prev_reachable=1
	while true; do
		local beat_ok=1
		_write_heartbeat 2>/dev/null || { warn "heartbeat write failed (will retry)"; beat_ok=0; }
		# Reconnect detection: previous beat failed, current beat succeeded
		if [[ "$prev_reachable" -eq 0 && "$beat_ok" -eq 1 ]]; then
			info "reconnect detected — running offline recovery"
			_run_offline_recovery
		fi
		prev_reachable="$beat_ok"
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

	if ! _daemon_up; then
		err "Daemon not reachable at $DAEMON_URL"
		err "Start the daemon first: ./daemon/start.sh"
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
	# Show last_seen for all peers from daemon API
	if ! _daemon_up; then
		warn "Daemon not reachable at $DAEMON_URL"
		return 0
	fi

	printf "  %-20s %-22s %-30s %s\n" "PEER" "LAST_SEEN" "LOAD" "CAPABILITIES"
	printf "  %-20s %-22s %-30s %s\n" "----" "---------" "----" "------------"

	local now json
	now="$(date +%s)"
	json=$(curl -sf "${DAEMON_URL}/api/heartbeat/status" 2>/dev/null || echo "")

	if [[ -n "$json" ]]; then
		echo "$json" | jq -r '.peers[]? | [.peer_name, (.last_seen // ""), (.load_json // "{}"), (.capabilities // "")] | @tsv' 2>/dev/null | while IFS=$'\t' read -r peer_name last_seen load_json caps; do
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
	fi

	echo ""
}

cmd_daemon() {
	# Foreground mode for service managers (launchd, systemd).
	# Runs the heartbeat loop WITHOUT forking — the OS manages the process.
	if ! _daemon_up; then
		err "Daemon not reachable at $DAEMON_URL"
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
