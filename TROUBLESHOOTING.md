# Troubleshooting

## Daemon / cvg CLI

**cvg not found**
- Cause: symlink missing. Fix: `scripts/platform/setup-claude-symlinks.sh` or
  `ln -sf "$(pwd)/daemon/target/release/convergio-platform-daemon" ~/.local/bin/cvg`

**"daemon not reachable on :8420"**
- Cause: daemon not running.
- Fix: `./daemon/start.sh` → `curl -s http://localhost:8420/api/health`
- Note: read-only commands work offline; writes require the daemon.

**sqlite3 WAL corruption / stale hooks**
- Cause: pre-Plan-685 hooks calling `sqlite3` directly.
- Fix: `./setup.sh` → verify `grep -c 'sqlite3' .claude/settings.json` = 0

**"daemon returns 405 on review/checkpoint endpoints"**
- Cause: running binary older than v12.1.0.
- Fix: `cd daemon && cargo build --release && ./daemon/start.sh`

**plan_reviews / checkpoints table missing**
- Cause: migrations not applied (old daemon running).
- Fix: restart daemon — migrations auto-apply on startup.

**cvg review reset fails "required argument PLAN_ID"**
- Fixed in v12.1.1. Rebuild daemon. `cvg review reset` now accepts no plan_id.

## Build

**cargo check fails**
- Fix: `cd daemon && cargo check 2>&1 | head -30` — read first error only.

**TUI crashes / garbled output**
- Fix: `TERM=xterm-256color cargo run -- tui`; check `data/logs/daemon-crash.log`.
  Run `reset` to restore terminal.

**TUI shows "No brain data"**
- Cause: no agents registered, or /ws/brain unreachable.
- Fix: `./daemon/start.sh` → `curl http://localhost:8420/api/agents`

## Setup

**setup.sh "claude-config not found"**
- Fix: run from repo root: `cd ~/GitHub/ConvergioPlatform && ./setup.sh`

**EnterPlanMode not blocked by hook**
- Fix: `test -f .claude/settings.json || ./setup.sh`
  Verify: `jq '.hooks.PreToolUse | length' .claude/settings.json` >= 8

**Skill sync shows 0 skills**
- Fix: `scripts/platform/agent-skills-sync.sh --platform-dir "$(pwd)"`

**Agent heartbeat stale**
- Fix: `scripts/platform/agent-heartbeat.sh --name <name> --task idle`

## Ingestion

**pdftotext not found** → `brew install poppler`
**pandoc not found** → `brew install pandoc`
**trafilatura not found** → `pip install trafilatura`

## Watchdog Alerts (W2 / Plan 724)

Watchdog runs every 30 s. Alerts dispatched via `notifications.conf`.

| Alert | Meaning | Action |
|---|---|---|
| `agent stalled >300s` | Task has no DB update in 5 min | Check task status: `cvg plan show <id>` |
| `/api/health/deep` non-healthy | A component is degraded | `curl -s http://localhost:8420/api/health/deep \| jq .` |
| `Ollama unavailable` | LLM summarisation off; rules-only mode | Install Ollama or ignore (fallback is automatic) |
| `rate limit detected` | Agent hit API rate limit | Check task log; watchdog will alert and block task |
| `wave complete` | Info notification on wave merge | No action needed |

**Configure ntfy.sh notifications:**
```toml
# claude-config/config/notifications.conf
[watchdog]
check_interval_secs = 30
ollama_url = "http://localhost:11434"
stale_threshold_secs = 300

[[notification_channels]]
type = "ntfy"
url = "https://ntfy.sh/your-topic"   # replace with your topic
```

**Test notification:** `cvg notify send "test" "hello" --severity info`

**Watchdog CLI:**
```bash
cvg watchdog start    # start background watchdog
cvg watchdog stop     # stop watchdog
cvg watchdog status   # show last check results
```

## Decision Log (F-27)

Every watchdog restart, reap, and block action is stored in `decision_log`.

```bash
cvg decision log                          # list recent decisions
cvg decision log --plan-id 724            # filter by plan
curl -s http://localhost:8420/api/decisions?plan_id=724 | jq .
```

Log a decision manually:
```bash
cvg decision log "chose retry over escalate" --reasoning "transient SQLITE_BUSY" --plan-id 724
```

## Zombie Reaper (F-25)

Reaper removes: stale worktrees (plan done > 24 h), merged branches, lock files > 1 h.
Auto-runs every 30 min. Manual:

```bash
cvg reap --dry-run    # preview only — no changes
cvg reap              # execute cleanup
```

What gets cleaned:
- `git worktree list` entries with no matching active wave
- `git branch --merged main` branches
- `/tmp/*.lock` files older than 1 hour

## Multi-Repo (W4 / Plan 724)

**Register a repo:**
```bash
cvg repo add convergio-daemon --path ~/GitHub/convergio-daemon \
  --github-url https://github.com/Roberdan/convergio-daemon
```

**List / inspect:**
```bash
cvg repo list           # all registered repos
cvg repo show convergio-daemon
```

**Link repo to project:**
```bash
cvg repo link convergio-daemon <project-id>
```

**Sync health for all repos:**
```bash
cvg repo sync           # checks each repo path exists + health endpoint responds
```

**Repo health unknown / not updating**
- Cause: daemon not running, or repo path missing.
- Fix: verify path exists, start daemon, re-run `cvg repo sync`.

## macOS / App

**Menu bar icon missing**
- Fix: `./daemon/start.sh` → rebuild app: `cd CommandCenter && ruby Scripts/generate_xcodeproj.rb`

**CommandCenter build uses CommandLineTools (wrong SDK)**
- Fix: `export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`

**PTY terminal rejects session name**
- Cause: name must match `[A-Za-z0-9_-]`, max 64 chars.

## Session Stability

**Session crashes / context limit reached**
- Symptom: "Context limit reached" or "cache_control.ttl ordering" error.
- Cause: Hook overhead (13 hooks per tool call) + inline merge conflict resolution.
- Fix: Use consolidated hooks (`scripts/platform/pre-tool-guard.sh`), delegate cherry-picks to agents, checkpoint after every task. See ADR-0113.

**Thor verify commands fail with "cd daemon: No such file or directory"**
- Symptom: `cvg task validate` returns REJECTED with "cd daemon" path error.
- Cause: Daemon process cwd is already `daemon/`, so `cd daemon &&` tries `daemon/daemon/`.
- Fix: Remove `cd daemon &&` prefix from verify commands in plan specs. Update task notes via API:
  `curl -s -X POST http://localhost:8420/api/plan-db/task/update -H "Content-Type: application/json" -d '{"task_id": ID, "status": "submitted", "notes": "cargo test FILTER"}'`

**plan-checkpoint.sh fails with "Cannot reach daemon"**
- Symptom: `plan-checkpoint.sh save` errors.
- Cause: v1 used wrong API path or `sqlite3` directly.
- Fix: Updated to v2.0 using `cvg plan show`. Rebuild: `cd daemon && cargo build --release`.

## Plan Workflow

**"deliverable not found"** → `cvg deliverable list --project <id>`

**workspace create fails** → start daemon, check parent dir permissions.

**Release pipeline stuck** → set `GITHUB_TOKEN`, run quality gate manually first.

**file_sizes gate fails** → split `.rs` files exceeding 250 lines into submodules.

**OpenClaw cannot reach daemon** → `./daemon/start.sh`; `curl http://localhost:8420/api/health`
