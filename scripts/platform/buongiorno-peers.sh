#!/usr/bin/env bash
# buongiorno-peers.sh — Peer update logic for buongiorno morning routine
# Sourced by buongiorno.sh. Requires: peers.sh, news array, color vars.

_buongiorno_update_peers() {
	[[ -f "$HOME/.claude/scripts/lib/peers.sh" ]] || return 0
	# shellcheck source=/dev/null
	source "$HOME/.claude/scripts/lib/peers.sh"
	peers_load 2>/dev/null || return 0

	local local_peer peer_num peer_total
	local_peer="${CLAUDE_LOCAL_PEER:-$(peers_self 2>/dev/null)}"
	peer_num=0
	peer_total=0

	local _p
	for _p in ${_PEERS_ACTIVE:-}; do
		[[ -n "$local_peer" && "$_p" == "$local_peer" ]] && continue
		peer_total=$((peer_total + 1))
	done

	for _p in ${_PEERS_ACTIVE:-}; do
		[[ -n "$local_peer" && "$_p" == "$local_peer" ]] && continue
		peer_num=$((peer_num + 1))

		local p_route p_user p_dest p_os p_icon
		p_route="$(peers_best_route "$_p" 2>/dev/null || peers_get "$_p" ssh_alias 2>/dev/null)"
		p_user="$(peers_get "$_p" user 2>/dev/null || echo "")"
		p_dest="${p_user:+${p_user}@}${p_route}"
		p_os="$(peers_get "$_p" os 2>/dev/null || echo "linux")"
		p_icon="🐧"
		[[ "$p_os" == "macos" ]] && p_icon="🍎"

		[[ -z "$p_route" ]] && {
			echo -e "  ${C}[${peer_num}/${peer_total}]${N} ${p_icon} ${_p}: ${Y}route mancante, skip${N}"
			continue
		}

		echo -e "  ${C}[${peer_num}/${peer_total}]${N} ${p_icon} ${_p} (${p_os})..."
		if ! ssh -n -o ConnectTimeout=4 -o BatchMode=yes "$p_dest" true 2>/dev/null; then
			echo -e "    ${Y}⚠${N} ${_p} non raggiungibile, skip"
			continue
		fi
		echo -e "    Connesso via ${Y}${p_dest}${N}"

		local RPATH r_claude_ver r_claude_after r_copilot_ver r_copilot_after
		RPATH='export PATH="/opt/homebrew/bin:/usr/local/bin:$HOME/.local/bin:$PATH";'

		r_claude_ver=$(ssh -n "$p_dest" "${RPATH} claude --version 2>/dev/null" 2>/dev/null)
		if [[ -n "$r_claude_ver" ]]; then
			echo -e "    Claude: ${r_claude_ver}"
			if [[ "$p_os" == "linux" ]]; then
				ssh -n "$p_dest" "${RPATH} command -v npm >/dev/null 2>&1 && sudo npm install -g --force @anthropic-ai/claude-code@latest 2>&1 || echo 'npm missing'" 2>/dev/null | tail -2
			else
				ssh -n "$p_dest" "${RPATH} claude update 2>&1" 2>/dev/null | tail -2
			fi
			r_claude_after=$(ssh -n "$p_dest" "${RPATH} claude --version 2>/dev/null" 2>/dev/null)
			if [[ "$r_claude_ver" != "$r_claude_after" ]]; then
				news+=("${p_icon} Claude ${_p}: ${r_claude_ver} → ${r_claude_after}")
			else
				echo -e "    ${G}✓${N} Claude già aggiornato (${r_claude_after})"
			fi
		fi

		r_copilot_ver=$(ssh -n "$p_dest" "${RPATH} gh copilot --version 2>/dev/null | head -1" 2>/dev/null)
		if [[ -n "$r_copilot_ver" ]]; then
			echo -e "    ${G}✓${N} Copilot built-in (${r_copilot_ver})"
		else
			r_copilot_ver=$(ssh -n "$p_dest" "${RPATH} gh extension list 2>/dev/null | awk '/copilot/ {print \\\$3; exit}'" 2>/dev/null)
			if [[ -n "$r_copilot_ver" ]]; then
				echo -e "    Copilot ext: ${r_copilot_ver}"
				ssh -n "$p_dest" "${RPATH} gh extension upgrade gh-copilot 2>&1" 2>/dev/null | tail -2
				r_copilot_after=$(ssh -n "$p_dest" "${RPATH} gh extension list 2>/dev/null | awk '/copilot/ {print \\\$3; exit}'" 2>/dev/null)
				if [[ "$r_copilot_ver" != "$r_copilot_after" ]]; then
					news+=("${p_icon} Copilot ${_p}: ${r_copilot_ver} → ${r_copilot_after}")
				else
					echo -e "    ${G}✓${N} Copilot già aggiornato (${r_copilot_after})"
				fi
			else
				echo -e "    ${Y}⚠${N} Copilot non disponibile"
			fi
		fi

		if [[ "$p_os" == "macos" ]]; then
			echo -e "    Homebrew..."
			ssh -n "$p_dest" "${RPATH} command -v brew >/dev/null 2>&1 && brew update --quiet && brew upgrade --quiet && brew cleanup --quiet 2>&1 || echo 'brew missing'" 2>/dev/null | tail -3
			echo -e "    ${G}✓${N} Homebrew aggiornato"
		fi

		news+=("${p_icon} ${_p} allineato")
	done
}
