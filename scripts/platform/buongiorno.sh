#!/usr/bin/env bash
# buongiorno.sh — Morning routine: update all tools across the mesh
# Part of ConvergioPlatform. Runs on the master node, updates all peers.
set -euo pipefail

# --- Colors ---
G='\033[0;32m' Y='\033[1;33m' R='\033[0;31m' C='\033[0;36m' B='\033[1m' N='\033[0m'

# --- Config ---
_buongiorno_master_peer() {
	echo "${BUONGIORNO_MASTER_PEER:-m5max}"
}

_buongiorno_mesh_sync() {
	local sync_script="$HOME/.claude/scripts/mesh-sync.sh"

	if [[ ! -x "$sync_script" ]]; then
		echo "    ⚠ mesh-sync.sh non trovato, skip"
		return 1
	fi

	"$sync_script" 2>&1 | tail -5
}

_buongiorno_redirect_to_master() {
	local master_peer local_peer
	master_peer="$(_buongiorno_master_peer)"

	[[ -f "$HOME/.claude/scripts/lib/peers.sh" ]] || return 1
	# shellcheck source=/dev/null
	source "$HOME/.claude/scripts/lib/peers.sh"
	peers_load 2>/dev/null || return 1

	local_peer="${CLAUDE_LOCAL_PEER:-$(peers_self 2>/dev/null)}"
	[[ "$local_peer" == "$master_peer" ]] && return 1

	local master_route master_user master_dest
	master_route="$(peers_best_route "$master_peer" 2>/dev/null || peers_get "$master_peer" ssh_alias 2>/dev/null)" || return 1
	master_user="$(peers_get "$master_peer" user 2>/dev/null || echo "")"
	master_dest="${master_user:+${master_user}@}${master_route}"
	[[ -n "$master_dest" ]] || return 1

	echo "↪ questo nodo non è il master (${local_peer:-unknown}). Reindirizzo a ${master_peer}..."
	ssh -t -o BatchMode=yes "$master_dest" "zsh -ic 'buongiorno --no-master-redirect'"
	return $?
}

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

		r_copilot_ver=$(ssh -n "$p_dest" "${RPATH} gh extension list 2>/dev/null | awk '/copilot/ {print \\\$3; exit}'" 2>/dev/null)
		if [[ -n "$r_copilot_ver" ]]; then
			echo -e "    Copilot: ${r_copilot_ver}"
			ssh -n "$p_dest" "${RPATH} gh extension upgrade gh-copilot 2>&1" 2>/dev/null | tail -2
			r_copilot_after=$(ssh -n "$p_dest" "${RPATH} gh extension list 2>/dev/null | awk '/copilot/ {print \\\$3; exit}'" 2>/dev/null)
			if [[ "$r_copilot_ver" != "$r_copilot_after" ]]; then
				news+=("${p_icon} Copilot ${_p}: ${r_copilot_ver} → ${r_copilot_after}")
			else
				echo -e "    ${G}✓${N} Copilot già aggiornato (${r_copilot_after})"
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

# --- Main ---
main() {
	local no_master_redirect=0
	if [[ "${1:-}" == "--no-master-redirect" ]]; then
		no_master_redirect=1
		shift
	fi

	if [[ "$no_master_redirect" -eq 0 ]] && _buongiorno_redirect_to_master; then
		exit 0
	fi

	local start master_peer local_peer execution_mode
	start=$(date +%s)
	declare -a news=()
	master_peer="$(_buongiorno_master_peer)"
	if [[ -f "$HOME/.claude/scripts/lib/peers.sh" ]]; then
		# shellcheck source=/dev/null
		source "$HOME/.claude/scripts/lib/peers.sh"
		peers_load 2>/dev/null || true
		local_peer="${CLAUDE_LOCAL_PEER:-$(peers_self 2>/dev/null)}"
	fi
	execution_mode="MASTER"
	[[ "$no_master_redirect" -eq 1 ]] && execution_mode="REMOTE-RUN"

	echo ""
	echo -e "${B}☀️  Buongiorno! Aggiorno tutto...${N}"
	echo -e "   Nodo: ${C}${local_peer:-unknown}${N} | Master: ${C}${master_peer}${N} | Mode: ${B}${execution_mode}${N}"
	echo ""

	echo -e "${C}[1/6]${N} 🤖 Claude Code..."
	if command -v claude >/dev/null 2>&1; then
		local claude_before claude_after
		claude_before=$(claude --version 2>/dev/null)
		if claude update 2>&1 | tail -3; then
			claude_after=$(claude --version 2>/dev/null)
			if [[ "$claude_before" != "$claude_after" ]]; then
				news+=("🤖 Claude Code: ${claude_before} → ${claude_after}")
			else
				echo -e "  ${G}✓${N} già aggiornato (${claude_after})"
			fi
		else
			echo -e "  ${R}✗${N} aggiornamento fallito"
		fi
	else
		echo -e "  ${Y}⚠${N} claude non trovato"
	fi

	echo -e "${C}[2/6]${N} 🐙 GitHub Copilot CLI..."
	if command -v gh >/dev/null 2>&1; then
		local copilot_before copilot_after
		copilot_before=$(gh extension list 2>/dev/null | awk '/copilot/ {print $3; exit}')
		if gh extension upgrade gh-copilot 2>&1 | tail -2; then
			copilot_after=$(gh extension list 2>/dev/null | awk '/copilot/ {print $3; exit}')
			if [[ "$copilot_before" != "$copilot_after" ]]; then
				news+=("🐙 GH Copilot: ${copilot_before} → ${copilot_after}")
			else
				echo -e "  ${G}✓${N} già aggiornato (${copilot_after})"
			fi
		else
			echo -e "  ${R}✗${N} aggiornamento fallito"
		fi
	else
		echo -e "  ${Y}⚠${N} gh non trovato"
	fi

	echo -e "${C}[3/6]${N} 🍺 Homebrew..."
	if command -v brew >/dev/null 2>&1; then
		local outdated
		brew update --quiet 2>/dev/null
		outdated=$(brew outdated 2>/dev/null)
		if [[ -n "$outdated" ]]; then
			local count
			count=$(echo "$outdated" | wc -l | tr -d ' ')
			echo -e "  Aggiorno ${Y}${count}${N} pacchetti..."
			brew upgrade --quiet 2>&1 | tail -5
			news+=("🍺 Homebrew: aggiornati ${count} pacchetti")
		else
			echo -e "  ${G}✓${N} tutto aggiornato"
		fi
		brew cleanup --quiet 2>/dev/null
	else
		echo -e "  ${Y}⚠${N} brew non disponibile su questo host, skip"
	fi

	echo -e "${C}[4/6]${N} 🔧 GitHub CLI & estensioni..."
	if command -v gh >/dev/null 2>&1; then
		gh extension upgrade --all 2>&1 | grep -v "already up to date" | tail -5
		echo -e "  ${G}✓${N} fatto"
	else
		echo -e "  ${Y}⚠${N} gh non trovato"
	fi

	echo -e "${C}[5/6]${N} 🌐 .claude Mesh Sync + aggiornamento peer..."
	_buongiorno_mesh_sync
	_buongiorno_update_peers

	echo -e "${C}[6/6]${N} 🩺 Mesh Preflight (tools + auth + versioni)..."
	if [[ -x "$HOME/.claude/scripts/mesh-preflight.sh" ]]; then
		"$HOME/.claude/scripts/mesh-preflight.sh" 2>&1 || news+=("🩺 Mesh preflight: ISSUES FOUND — check dashboard")
	else
		echo -e "  ${Y}⚠${N} mesh-preflight.sh non trovato"
	fi

	local elapsed
	elapsed=$(( $(date +%s) - start ))
	echo ""
	echo -e "${B}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
	if [[ ${#news[@]} -gt 0 ]]; then
		echo -e "${B}📰 Novità di oggi:${N}"
		local item
		for item in "${news[@]}"; do
			echo -e "  • ${item}"
		done
	else
		echo -e "${G}✨ Tutto era già aggiornato!${N}"
	fi
	echo -e "${B}⏱  Completato in ${elapsed}s${N}"
	echo -e "${B}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
	echo ""
	echo -e "${G}☕ Buon lavoro, Roberto!${N}"
	echo ""
}

main "$@"
