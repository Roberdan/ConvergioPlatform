#!/bin/bash
# Version: 1.0.0
# Remote tmux launcher — ALL COPILOT (zero Claude tokens)
# Runs on the LINUX machine
set -euo pipefail

_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../../config/load-config.sh
source "$_SCRIPT_DIR/../../config/load-config.sh" 2>/dev/null || true
unset _SCRIPT_DIR

CLAUDE_HOME=${CLAUDE_HOME:-$HOME/.claude}

SESSION="audit-remediation"

_GH_DEFAULT="${GH_DEFAULT_ACCOUNT:-Roberdan}"
[[ -z "${GH_MICROSOFT_ACCOUNT:-}" ]] && { echo "ERROR: GH_MICROSOFT_ACCOUNT not set. Run: cp config/local.env.example config/local.env" >&2; exit 1; }
_GH_MICROSOFT="${GH_MICROSOFT_ACCOUNT}"

# Resolve GH tokens for multi-account repos
MAIN_TOKEN=$(gh auth switch --user "$_GH_DEFAULT" 2>/dev/null && gh auth token 2>/dev/null || echo "")
gh auth switch --user "$_GH_MICROSOFT" 2>/dev/null || true
VBPM_TOKEN=$(gh auth token 2>/dev/null || echo "")
gh auth switch --user "$_GH_DEFAULT" 2>/dev/null || true

# plan_id|project|label|dir|gh_token
PLANS=(
	"265|mirrorbuddy|MirrorBuddy|${HOME}/GitHub/MirrorBuddy|$MAIN_TOKEN"
	"266|virtualbpm|VirtualBPM|${HOME}/GitHub/VirtualBPM|$VBPM_TOKEN"
	"267|myconvergio|MyConvergio|${HOME}/GitHub/MyConvergio|$MAIN_TOKEN"
	"268|claude-global|Claude-Global|${CLAUDE_HOME}|$MAIN_TOKEN"
)

# Kill existing session if any
tmux kill-session -t "$SESSION" 2>/dev/null || true

# Create session with first plan
IFS='|' read -r pid project label dir token <<<"${PLANS[0]}"
tmux new-session -d -s "$SESSION" -n "$label" -c "$dir"
tmux send-keys -t "$SESSION:$label" \
	"export GH_TOKEN='$token' && echo '=== Plan #$pid: $label (COPILOT) ===' && cd $dir && copilot --yolo -p '@execute $pid'" Enter

# Create remaining windows
for i in 1 2 3; do
	IFS='|' read -r pid project label dir token <<<"${PLANS[$i]}"
	tmux new-window -t "$SESSION" -n "$label" -c "$dir"
	tmux send-keys -t "$SESSION:$label" \
		"export GH_TOKEN='$token' && echo '=== Plan #$pid: $label (COPILOT) ===' && cd $dir && copilot --yolo -p '@execute $pid'" Enter
done

# Create monitor window (5th tab)
tmux new-window -t "$SESSION" -n "Monitor" -c "${HOME}"
tmux send-keys -t "$SESSION:Monitor" \
	"export PATH=\"\$HOME/.claude/scripts:\$PATH\" && pianits" Enter

# Select first window
tmux select-window -t "$SESSION:1"

echo "Tmux session '$SESSION' created with 5 windows (ALL COPILOT):"
echo "  1: MirrorBuddy   (Plan #265) — GH: $_GH_DEFAULT"
echo "  2: VirtualBPM    (Plan #266) — GH: $_GH_MICROSOFT"
echo "  3: MyConvergio   (Plan #267) — GH: $_GH_DEFAULT"
echo "  4: Claude-Global (Plan #268) — GH: $_GH_DEFAULT"
echo "  5: Monitor       (pianits terminal dashboard)"
echo ""
echo "Attach: tmux attach -t $SESSION"
