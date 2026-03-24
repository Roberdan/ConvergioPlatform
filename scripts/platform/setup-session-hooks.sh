#!/usr/bin/env bash
# setup-session-hooks.sh — Inject agent registration hooks into Claude Code and Copilot CLI
# Source of truth: ConvergioPlatform. Only symlink-weight references in tool configs.
# Usage: setup-session-hooks.sh [install|remove]
# Called by: convergio-toggle.sh on/off, setup.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOOK_SCRIPT="$SCRIPT_DIR/agent-session-hook.sh"

# Tool config paths
CLAUDE_SETTINGS="$HOME/.claude/settings.json"
COPILOT_SETTINGS="$HOME/.copilot/config.json"

require_jq() {
  if ! command -v python3 &>/dev/null; then
    echo "ERROR: python3 required for JSON manipulation" >&2
    exit 1
  fi
}

# Inject hooks into a JSON settings file using python3 (jq-free)
inject_hooks() {
  local settings_file="$1"
  local agent_type="$2"

  if [ ! -f "$settings_file" ]; then
    echo "  SKIP: $settings_file not found"
    return
  fi

  # Check if hooks already configured
  if python3 -c "
import json, sys
d = json.load(open('$settings_file'))
hooks = d.get('hooks', {})
for h in hooks.get('SessionStart', []):
    for sub in h.get('hooks', []):
        if 'agent-session-hook' in sub.get('command', ''):
            sys.exit(0)
sys.exit(1)
" 2>/dev/null; then
    echo "  OK: $settings_file (hooks already present)"
    return
  fi

  # Backup before modifying
  cp "$settings_file" "${settings_file}.convergio-backup"

  python3 -c "
import json

with open('$settings_file') as f:
    d = json.load(f)

hooks = d.setdefault('hooks', {})

start_hook = {
    'hooks': [{
        'type': 'command',
        'command': '$HOOK_SCRIPT start $agent_type',
        'timeout': 5,
        'async': True
    }]
}

stop_hook = {
    'hooks': [{
        'type': 'command',
        'command': '$HOOK_SCRIPT complete $agent_type',
        'timeout': 5
    }]
}

# Append to existing hook arrays (don't overwrite other hooks)
hooks.setdefault('SessionStart', []).append(start_hook)
hooks.setdefault('Stop', []).append(stop_hook)

with open('$settings_file', 'w') as f:
    json.dump(d, f, indent=2)
    f.write('\n')
"
  echo "  OK: $settings_file (hooks injected, backup at ${settings_file}.convergio-backup)"
}

# Remove hooks from a JSON settings file
remove_hooks() {
  local settings_file="$1"

  if [ ! -f "$settings_file" ]; then
    return
  fi

  # Check if our hooks exist
  if ! grep -q 'agent-session-hook' "$settings_file" 2>/dev/null; then
    echo "  OK: $settings_file (no hooks to remove)"
    return
  fi

  python3 -c "
import json

with open('$settings_file') as f:
    d = json.load(f)

hooks = d.get('hooks', {})

def remove_ours(hook_list):
    return [
        entry for entry in hook_list
        if not any(
            'agent-session-hook' in sub.get('command', '')
            for sub in entry.get('hooks', [])
        )
    ]

for event in ['SessionStart', 'Stop', 'SessionEnd']:
    if event in hooks:
        hooks[event] = remove_ours(hooks[event])
        if not hooks[event]:
            del hooks[event]

if not hooks:
    del d['hooks']

with open('$settings_file', 'w') as f:
    json.dump(d, f, indent=2)
    f.write('\n')
"
  echo "  OK: $settings_file (hooks removed)"
}

install() {
  echo "=== Session Hook Setup ==="
  require_jq

  # Ensure hook script exists and is executable
  if [ ! -x "$HOOK_SCRIPT" ]; then
    echo "ERROR: $HOOK_SCRIPT not found or not executable" >&2
    exit 1
  fi

  inject_hooks "$CLAUDE_SETTINGS" "claude"
  inject_hooks "$COPILOT_SETTINGS" "copilot"

  echo ""
  echo "Agent sessions will auto-register with daemon on start/stop."
  echo "Verify: cvg who agents"
}

remove() {
  echo "=== Session Hook Teardown ==="

  remove_hooks "$CLAUDE_SETTINGS"
  remove_hooks "$COPILOT_SETTINGS"

  # Restore backups if they exist
  for f in "$CLAUDE_SETTINGS" "$COPILOT_SETTINGS"; do
    if [ -f "${f}.convergio-backup" ]; then
      echo "  Backup available: ${f}.convergio-backup"
    fi
  done

  echo ""
  echo "Session hooks removed. Agent sessions will no longer auto-register."
}

case "${1:-install}" in
  install|on)   install ;;
  remove|off)   remove ;;
  *)
    echo "Usage: setup-session-hooks.sh [install|remove]"
    exit 1
    ;;
esac
