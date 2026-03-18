#!/usr/bin/env bash
# peers.sh — Peer discovery library (sourced, not executed)
# NOTE: No set -euo pipefail — this is a sourced library, callers set their own error handling
# Version: 1.0.0
# Requires: bash 3.2+, ssh. Source this file, then call peers_load.
# Usage: source scripts/lib/peers.sh && peers_load && peers_list

# Guard: prevent direct execution (skip in zsh where BASH_SOURCE is unset)
if [[ -n "${BASH_SOURCE[0]:-}" && "${BASH_SOURCE[0]}" == "${0}" ]]; then
	echo "ERROR: peers.sh must be sourced." >&2
	exit 1
fi

# zsh compat: enable word splitting on unquoted $var (bash default)
[[ -n "${ZSH_VERSION:-}" ]] && setopt SH_WORD_SPLIT 2>/dev/null

PEERS_CONF="${PEERS_CONF:-${CLAUDE_HOME:-$HOME/.claude}/config/peers.conf}"
_PEERS_ALL=""    # space-separated list of all peer names
_PEERS_ACTIVE="" # space-separated list of active peer names

# Internal: store peer field value (zero subshells — all bash builtins)
_peers_set() {
	local _n="${1//[-.]/_}"
	[[ "${BASH_VERSINFO[0]:-3}" -ge 4 ]] && _n="${_n^^}" || _n=$(printf '%s' "$_n" | tr '[:lower:]' '[:upper:]')
	local _val="${3//\'/\'\\\'\'}"
	eval "_PEER_${_n}_${2}='${_val}'"
}

# Internal: retrieve peer field value (zero subshells)
_peers_get_raw() {
	local _n="${1//[-.]/_}"
	[[ "${BASH_VERSINFO[0]:-3}" -ge 4 ]] && _n="${_n^^}" || _n=$(printf '%s' "$_n" | tr '[:lower:]' '[:upper:]')
	eval "printf '%s' \"\${_PEER_${_n}_${2}:-}\""
}

# peers_load — parse peers.conf into internal state
# peers_load — parse peers.conf into internal state (with mtime-based cache)
peers_load() {
	_PEERS_ALL=""
	_PEERS_ACTIVE=""
	if [[ ! -f "$PEERS_CONF" ]]; then
		echo "ERROR: peers.conf not found: $PEERS_CONF" >&2
		return 1
	fi

	# mtime-based cache: skip re-parsing if file unchanged
	local cache_file="${TMPDIR:-/tmp}/peers-cache-$$"
	local conf_mtime
	conf_mtime=$(stat -c %Y "$PEERS_CONF" 2>/dev/null || stat -f %m "$PEERS_CONF" 2>/dev/null || echo 0)
	local global_cache="${TMPDIR:-/tmp}/peers-cache-mtime-${conf_mtime}"

	if [[ -f "$global_cache" ]]; then
		# Source cached variable assignments
		# shellcheck disable=SC1090
		source "$global_cache" 2>/dev/null && [[ -n "$_PEERS_ALL" ]] && return 0
	fi

	local current_peer="" line key val
	while IFS= read -r line || [[ -n "$line" ]]; do
		line="${line%%#*}"
		line="${line#"${line%%[! 	]*}"}"
		line="${line%"${line##*[! 	]}"}"
		[[ -z "$line" ]] && continue
		if [[ "$line" == \[*\] ]]; then
			current_peer="${line#\[}"
			current_peer="${current_peer%\]}"
			_PEERS_ALL="${_PEERS_ALL:+$_PEERS_ALL }$current_peer"
			_peers_set "$current_peer" "status" "active"
			continue
		fi
		if [[ -n "$current_peer" && "$line" == *"="* ]]; then
			key="${line%%=*}"
			val="${line#*=}"
			_peers_set "$current_peer" "$key" "$val"
		fi
	done <"$PEERS_CONF"
	local name st role
	for name in $_PEERS_ALL; do
		# Skip non-peer sections (e.g. [mesh] global config — no role field)
		role="$(_peers_get_raw "$name" "role")"
		[[ -z "$role" ]] && continue
		st="$(_peers_get_raw "$name" "status")"
		[[ "$st" == "active" ]] && _PEERS_ACTIVE="${_PEERS_ACTIVE:+$_PEERS_ACTIVE }$name"
	done

	# Write cache: dump all _PEER_* variables + _PEERS_ALL/_PEERS_ACTIVE
	{
		echo "_PEERS_ALL='$_PEERS_ALL'"
		echo "_PEERS_ACTIVE='$_PEERS_ACTIVE'"
		# Dump _PEER_ variables — inline key computation (no function calls)
		local _pvar _pn _field _val
		for _pvar in $_PEERS_ACTIVE; do
			_pn="${_pvar//[-.]/_}"
			if [[ "${BASH_VERSINFO[0]:-3}" -ge 4 ]]; then
				_pn="${_pn^^}"
			else
				_pn=$(printf '%s' "$_pn" | tr '[:lower:]' '[:upper:]')
			fi
			for _field in ssh_alias user os tailscale_ip dns_name capabilities role status mac_address gh_account runners runner_paths shared_secret default_engine; do
				eval "_val=\"\${_PEER_${_pn}_${_field}:-}\""
				[[ -n "$_val" ]] && echo "_PEER_${_pn}_${_field}='${_val}'"
			done
		done
	} > "$global_cache" 2>/dev/null || true
	# Clean old caches (different mtime)
	find "${TMPDIR:-/tmp}" -maxdepth 1 -name "peers-cache-mtime-*" ! -name "peers-cache-mtime-${conf_mtime}" -delete 2>/dev/null || true
}

# peers_list — echo active peer names, one per line
peers_list() {
	local name
	for name in $_PEERS_ACTIVE; do echo "$name"; done
}

# peers_get name field — return field value for named peer
peers_get() {
	local name="${1:-}" field="${2:-}" val
	if [[ -z "$name" || -z "$field" ]]; then
		echo "Usage: peers_get <name> <field>" >&2
		return 1
	fi
	val="$(_peers_get_raw "$name" "$field")"
	[[ -n "$val" ]] && echo "$val" || return 1
}

# peers_engine name — return default_engine for peer (empty if not set)
peers_engine() {
	_peers_get_raw "${1:-}" "default_engine"
}

# peers_check name — SSH connectivity check; returns 0=reachable, 1=not
peers_check() {
	local name="${1:-}"
	[[ -z "$name" ]] && {
		echo "Usage: peers_check <name>" >&2
		return 1
	}
	local target user dest
	target="$(peers_best_route "$name")" || return 1
	[[ -z "$target" ]] && return 1
	user="$(_peers_get_raw "$name" "user")"
	dest="${user:+${user}@}${target}"
	ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=accept-new \
		-o BatchMode=yes -o LogLevel=quiet "$dest" true >/dev/null 2>&1
}

# peers_online — echo reachable active peer names
peers_online() {
	local name
	for name in $_PEERS_ACTIVE; do
		peers_check "$name" 2>/dev/null && echo "$name"
	done
}

# peers_with_capability cap — active peers with capability in their list
peers_with_capability() {
	local cap="${1:-}"
	[[ -z "$cap" ]] && {
		echo "Usage: peers_with_capability <capability>" >&2
		return 1
	}
	local name caps
	for name in $_PEERS_ACTIVE; do
		caps="$(_peers_get_raw "$name" "capabilities")"
		case ",$caps," in *",${cap},"*) echo "$name" ;; esac
	done
}

# peers_best_route name — try ssh_alias first, fallback tailscale_ip
peers_best_route() {
	local name="${1:-}"
	[[ -z "$name" ]] && {
		echo "Usage: peers_best_route <name>" >&2
		return 1
	}
	local alias ts_ip
	alias="$(_peers_get_raw "$name" "ssh_alias")"
	ts_ip="$(_peers_get_raw "$name" "tailscale_ip")"
	if [[ -n "$alias" ]]; then
		echo "$alias"
	elif [[ -n "$ts_ip" ]]; then
		echo "$ts_ip"
	else return 1; fi
}

# peers_self — detect current machine by matching hostname or Tailscale IP
peers_self() {
	local current_host name alias ts_ip
	current_host="$(hostname -s 2>/dev/null || hostname)"
	# Method 1: hostname match
	for name in $_PEERS_ALL; do
		alias="$(_peers_get_raw "$name" "ssh_alias")"
		if [[ "$alias" == "$current_host" || "$name" == "$current_host" ]]; then
			echo "$name"
			return 0
		fi
	done
	# Method 2: Tailscale IP match (handles macOS hostname mismatch)
	local local_ts_ip
	local_ts_ip="$(tailscale ip --4 2>/dev/null || true)"
	if [[ -n "$local_ts_ip" ]]; then
		for name in $_PEERS_ALL; do
			ts_ip="$(_peers_get_raw "$name" "tailscale_ip")"
			[[ "$ts_ip" == "$local_ts_ip" ]] && {
				echo "$name"
				return 0
			}
		done
	fi
	return 0
}

# peers_others — active peers excluding self
peers_others() {
	local self name
	self="$(peers_self)"
	for name in $_PEERS_ACTIVE; do
		[[ "$name" != "$self" ]] && echo "$name"
	done
}

# Translate Unix path to Windows path for remote commands
_remote_claude_home() {
	local peer="$1"
	local os
	os="$(peers_get "$peer" "os" 2>/dev/null || echo "linux")"
	if [[ "$os" == "windows" ]]; then
		echo '%USERPROFILE%\.claude'
	else
		echo '~/.claude'
	fi
}
