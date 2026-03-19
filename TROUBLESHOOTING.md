# Troubleshooting

## Problem: setup.sh fails with "claude-config not found"

**Symptom:** Running `./setup.sh` exits with "ERROR: claude-config not found"
**Cause:** ConvergioPlatform not cloned correctly or script run from wrong directory
**Fix:**
```bash
cd ~/GitHub/ConvergioPlatform  # or wherever you cloned
ls claude-config/              # must exist
./setup.sh
```

## Problem: Agent not registering in IPC

**Symptom:** `agent-bridge.sh --register` warns "daemon not reachable" to stderr
**Cause:** Daemon not running on port 8420
**Fix:**
```bash
# Check daemon status
curl -s http://localhost:8420/api/ipc/status
# If not running, start it
./daemon/start.sh
# Retry registration
scripts/platform/agent-bridge.sh --register --name test --type claude
# Verify
curl -s http://localhost:8420/api/ipc/agents | jq '.agents'
```

## Problem: Hooks not firing (EnterPlanMode not blocked)

**Symptom:** Can use EnterPlanMode without getting blocked by guard-plan-mode hook
**Cause:** `.claude/settings.json` missing from project root or not loaded
**Fix:**
```bash
# Check project-level settings
test -f .claude/settings.json && echo "exists" || echo "MISSING"
# If missing, run setup
./setup.sh
# Verify hooks
jq '.hooks.PreToolUse | length' .claude/settings.json
# Should be >= 8
```

## Problem: Skill sync shows 0 skills

**Symptom:** `agent-skills-sync.sh` runs but reports "Synced 0 skills"
**Cause:** claude-core binary not on PATH, or DB not accessible, or commands/ dir not found
**Fix:**
```bash
# Check claude-core
which claude-core || echo "NOT ON PATH"
# Check commands dir
ls claude-config/commands/*.md | wc -l  # should be >= 8
# Check DB
echo $DASHBOARD_DB
sqlite3 "$DASHBOARD_DB" "SELECT count(*) FROM ipc_agent_skills;" 2>/dev/null
# Re-run with explicit path
scripts/platform/agent-skills-sync.sh --platform-dir "$(pwd)"
```

## Problem: Agent heartbeat missing / stale

**Symptom:** Agent shows old `last_heartbeat` in GET /api/ipc/agents
**Cause:** Heartbeat script not running, or daemon was down during heartbeat
**Fix:**
```bash
# Manual heartbeat
scripts/platform/agent-heartbeat.sh --name <agent-name> --task idle
# Check result
curl -s http://localhost:8420/api/ipc/agents | jq '.agents[] | select(.agent_id=="<agent-name>") | .last_heartbeat'
# For persistent heartbeat, set up cron:
# */1 * * * * /path/to/scripts/platform/agent-heartbeat.sh --name myagent
```

## Problem: MyConvergio references after migration to ConvergioPlatform

**Symptom:** Scripts, configs, or docs still reference `MyConvergio`, `sync-to-myconvergio-ops.sh`, or old repo paths after the Plan #671 consolidation.
**Cause:** MyConvergio was merged into ConvergioPlatform; stale references were not fully cleaned up.
**Fix:**
```bash
# Search for remaining references
grep -ri 'myconvergio' scripts/ daemon/ dashboard/ claude-config/ || echo "Clean"
# Verify sync script is gone
test -f claude-config/scripts/lib/sync-to-myconvergio-ops.sh && echo "DELETE IT" || echo "Already removed"
# Verify provisioning uses ConvergioPlatform paths
grep -q 'ConvergioPlatform' scripts/mesh/mesh-provision-node.sh && echo "OK" || echo "Update paths"
# The canonical repo is ConvergioPlatform — update any bookmarks or CI configs
```

## Problem: Menu Bar not showing in system tray

**Symptom:** ConvergioMissionControl app runs but no menu bar icon appears
**Cause:** App not built as LSUIElement (agent app) or daemon not running on :8420
**Fix:**
```bash
# Verify daemon is running
curl -s http://localhost:8420/api/ipc/status | jq .
# Rebuild menu bar app
cd gui/ConvergioMissionControl && xcodebuild -scheme ConvergioMissionControl build
# Check Info.plist has LSUIElement = YES
defaults read gui/ConvergioMissionControl/Info.plist LSUIElement
```

## Problem: TUI fails to start or renders incorrectly

**Symptom:** `cargo run -- tui` exits immediately or shows garbled output
**Cause:** Terminal does not support alternate screen, or daemon binary not built
**Fix:**
```bash
# Ensure release build exists
cd daemon && cargo build --release
# Run TUI with explicit terminal
TERM=xterm-256color cargo run -- tui
# If still broken, check ratatui version
grep ratatui Cargo.toml
```

## Problem: Evolution proposals not loading in dashboard

**Symptom:** Evolution section shows empty or spinner, console shows 500 error
**Cause:** `evolution_proposals` table not yet created (auto-created on first API call) or DB path wrong
**Fix:**
```bash
# Trigger table creation
curl -s http://localhost:8420/api/evolution/proposals | jq .
# Check DB has the table
sqlite3 "$DASHBOARD_DB" ".tables" | grep evolution
# If missing, the GET call above creates it; retry dashboard
```

## Problem: Copilot agent not visible in /api/ipc/agents

**Symptom:** `copilot-bridge.sh --register` succeeds but GET /api/ipc/agents shows empty
**Cause:** Script may be using old /api/ipc/send path instead of /api/ipc/agents/register
**Fix:**
```bash
# Verify which endpoint is being called
bash -x scripts/platform/copilot-bridge.sh --register --name test-copilot 2>&1 | grep curl
# Should show: /api/ipc/agents/register
# Manual test
curl -X POST http://localhost:8420/api/ipc/agents/register \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"test-copilot","host":"'$(hostname)'"}'
curl -s http://localhost:8420/api/ipc/agents | jq '.agents'
```
