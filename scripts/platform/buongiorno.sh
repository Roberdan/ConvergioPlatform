#!/usr/bin/env bash
# buongiorno.sh — Morning routine: aggiorna i tool locali
set -euo pipefail

# --- Colors ---
G='\033[0;32m' Y='\033[1;33m' R='\033[0;31m' C='\033[0;36m' B='\033[1m' N='\033[0m'

# --- Main ---
main() {
	local start
	start=$(date +%s)
	declare -a news=()

	echo ""
	echo -e "${B}☀️  Buongiorno! Aggiorno i tool locali...${N}"
	echo ""

	echo -e "${C}[1/4]${N} 🤖 Claude Code..."
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

	echo -e "${C}[2/4]${N} 🐙 GitHub Copilot CLI..."
	if command -v gh >/dev/null 2>&1; then
		local copilot_before copilot_after
		if gh copilot --version >/dev/null 2>&1; then
			copilot_before=$(gh copilot --version 2>/dev/null | head -1)
			echo -e "  ${G}✓${N} built-in (${copilot_before})"
		elif gh extension list 2>/dev/null | grep -q copilot; then
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
			echo -e "  ${Y}⚠${N} copilot non disponibile (né built-in né extension)"
		fi
	else
		echo -e "  ${Y}⚠${N} gh non trovato"
	fi

	echo -e "${C}[3/4]${N} 🍺 Homebrew..."
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

	echo -e "${C}[4/4]${N} 🔧 GitHub CLI & estensioni..."
	if command -v gh >/dev/null 2>&1; then
		local gh_ext_output
		gh_ext_output=$(gh extension upgrade --all 2>&1) || true
		if [[ -n "$gh_ext_output" ]]; then
			echo "$gh_ext_output" | command grep -v "already up to date" | tail -5
		fi
		echo -e "  ${G}✓${N} fatto"
	else
		echo -e "  ${Y}⚠${N} gh non trovato"
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