#!/usr/bin/env bash
# Bootstrap system agents for both GitHub Copilot CLI and Claude Code
# Run on any new machine after cloning the repo
set -euo pipefail

REPO_AGENTS="$(cd "$(dirname "$0")/.." && pwd)/.github/agents"
COPILOT_AGENTS="$HOME/.copilot/agents"
CLAUDE_AGENTS="$HOME/.claude/agents/core_utility"

echo "=== Bootstrapping agents from $REPO_AGENTS ==="

# GitHub Copilot CLI
mkdir -p "$COPILOT_AGENTS"
for f in planner execute validate check prompt code-reviewer; do
  src="$REPO_AGENTS/${f}.agent.md"
  if [[ -f "$src" ]]; then
    cp "$src" "$COPILOT_AGENTS/"
    echo "  [copilot] $f.agent.md"
  fi
done

# Claude Code — copy as plan-* names to fit claude agent naming
mkdir -p "$CLAUDE_AGENTS"
declare -A CLAUDE_MAP=(
  ["planner"]="plan-reviewer.md"
  ["validate"]="thor-quality-assurance-guardian.md"
)
for copilot_name in "${!CLAUDE_MAP[@]}"; do
  src="$REPO_AGENTS/${copilot_name}.agent.md"
  dst="$CLAUDE_AGENTS/${CLAUDE_MAP[$copilot_name]}"
  if [[ -f "$src" && ! -f "$dst" ]]; then
    echo "  [claude] $copilot_name -> ${CLAUDE_MAP[$copilot_name]} (skipped, already exists)"
  fi
done

echo "=== Done. Agents available on this machine. ==="
