#!/usr/bin/env bash
# Buongiorno functions for shell aliases
# Sourced from shell-aliases.sh — extracted to meet 250-line limit

_buongiorno_mesh_sync() {
	# mesh-sync.sh replaced by daemon auto-sync
	if command -v cvg &>/dev/null; then
		cvg mesh sync 2>/dev/null || echo "    ⚠ daemon not running, mesh sync skip"
	else
		echo "    ⚠ cvg CLI not found, mesh sync skip"
		return 1
	fi
}

_buongiorno_master_peer() {
	echo "${BUONGIORNO_MASTER_PEER:-m5max}"
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

claude_buongiorno() {
	local G='\033[0;32m' Y='\033[1;33m' R='\033[0;31m' C='\033[0;36m' B='\033[1m' N='\033[0m'
	local no_master_redirect=0
	if [[ "${1:-}" == "--no-master-redirect" ]]; then
		no_master_redirect=1
		shift
	fi

	if [[ "$no_master_redirect" -eq 0 ]] && _buongiorno_redirect_to_master; then
		return 0
	fi

	local start master_peer local_peer execution_mode
	start=$(date +%s)
	local -a news=()
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

	echo -e "${C}[1/5]${N} 🤖 Claude Code..."
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

	echo -e "${C}[2/5]${N} 🐙 GitHub Copilot CLI..."
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

	echo -e "${C}[3/5]${N} 🍺 Homebrew..."
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

	echo -e "${C}[4/5]${N} 🔧 GitHub CLI & estensioni..."
	if command -v gh >/dev/null 2>&1; then
		gh extension upgrade --all 2>&1 | grep -v "already up to date" | tail -5
		echo -e "  ${G}✓${N} fatto"
	else
		echo -e "  ${Y}⚠${N} gh non trovato"
	fi

	echo -e "${C}[5/5]${N} 🌐 Mesh Sync..."
	_buongiorno_mesh_sync

	echo -e "${C}[6/6]${N} 🩺 Mesh Preflight..."
	if [[ -x "$HOME/.claude/scripts/mesh-preflight.sh" ]]; then
		"$HOME/.claude/scripts/mesh-preflight.sh" 2>&1 || news+=("🩺 Mesh preflight: ISSUES FOUND")
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

buongiorno() {
	claude_buongiorno "$@"
}
