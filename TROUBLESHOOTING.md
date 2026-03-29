# Troubleshooting

## Silent Error Patterns (fail-loud policy, ADR-0124)

**Problem: errors silently swallowed, tasks appear "done" but aren't**
- Symptom: task marked done but feature doesn't work; no errors in logs
- Cause: `.ok()` or `let _ =` patterns dropping errors without logging
- Fix: Plan 756 eliminated 413 of 446 patterns. Remaining 33 are annotated with `// intentional: <reason>`
- Prevention: `cargo clippy -- -W clippy::let_underscore_drop`; wiring check hook catches new `.rs` files without `mod` declaration

**Problem: notification endpoint returns success but message not delivered**
- Symptom: `POST /api/notify` returns `{"ok": true}` but no notification arrives
- Cause: (FIXED in Plan 756) handler returned hardcoded success regardless of delivery result
- Fix: endpoint now returns per-channel status: `{"ok": true, "channels": {"telegram": "sent", "ntfy": "error: timeout"}}`

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

Watchdog runs every 30s. Alerts via `notifications.conf`. Test: `cvg notify send "test" "hello" --severity info`

| Alert | Action |
|---|---|
| `agent stalled >300s` | `cvg plan show <id>` |
| `/api/health/deep` non-healthy | `curl -s http://localhost:8420/api/health/deep \| jq .` |
| `Ollama unavailable` | Install Ollama or ignore (fallback automatic) |
| `rate limit detected` | Check task log; watchdog blocks task |

CLI: `cvg watchdog start|stop|status`
Config: `claude-config/config/notifications.conf` (set ntfy topic, check_interval, stale_threshold)

## Decision Log (F-27)

Every watchdog restart, reap, and block action is stored in `decision_log`.
Query: `cvg decision log [--plan-id 724]` or `GET /api/decisions?plan_id=724`
Log manually: `cvg decision log "message" --reasoning "reason" --plan-id 724`

## Zombie Reaper (F-25)

Reaper removes: stale worktrees (plan done > 24h), merged branches, lock files > 1h.
Auto-runs every 30 min. Manual: `cvg reap --dry-run` (preview) | `cvg reap` (execute).

## Multi-Repo (W4 / Plan 724)

Commands: `cvg repo add <name> --path <p> [--github-url <u>]` | `cvg repo list` | `cvg repo show <name>` | `cvg repo link <name> <project-id>` | `cvg repo sync`

**Repo health unknown / not updating**
- Cause: daemon not running, or repo path missing.
- Fix: verify path exists, start daemon, re-run `cvg repo sync`.

## Kernel (Plan 729)

**Kernel model won't load**
- Symptom: `cvg kernel start` exits immediately or prints "model load failed".
- Cause: `mlx_lm` Python package not installed, or insufficient disk space for model (~5 GB).
- Fix: `pip install mlx-lm` → verify `python3 -c "import mlx_lm"` succeeds.
  Check disk: `df -h ~/.cache/huggingface` → need ≥ 6 GB free.

**Telegram not responding**
- Symptom: kernel starts but no Telegram messages arrive.
- Cause: `CONVERGIO_TELEGRAM_TOKEN` env var not set in daemon environment.
- Fix: `echo $CONVERGIO_TELEGRAM_TOKEN` → if empty, add to `.env` and restart daemon:
  `./daemon/start.sh`

**Audio not playing on active node**
- Symptom: voice alerts play on M1 Pro (kernel node) instead of current working node.
- Cause: `cvg kernel here` not run on the active node, or mesh partition.
- Fix: On the node where you are working, run `cvg kernel here` to register it as audio target.
  Verify: `cvg kernel status` shows correct `audio_target` node.
  Fallback: `afplay` must be available (`which afplay`) for local audio output.

## Nightly Calibration (Plan 734)

**calibrate-models.sh not found on M1 Pro**
- Symptom: `bash: scripts/kernel/calibrate-models.sh: No such file or directory`
- Cause: M1 Pro repo is behind main (script added in Plan T commit `3bba32f`).
- Fix: `ssh m1Pro "cd ~/GitHub/ConvergioPlatform && git pull --rebase origin main"`

**Telegram notification skipped**
- Symptom: `[calibrate-models] Telegram not configured — skipping notification`
- Cause: `CONVERGIO_TELEGRAM_TOKEN` or `CONVERGIO_TELEGRAM_CHAT_ID` not set in `~/.convergio/env`.
- Fix: add both vars to `~/.convergio/env` on M1 Pro and reload: `source ~/.convergio/env`.

**Evolution proposal submission fails (non-fatal)**
- Symptom: `[calibrate-models] Evolution proposal submission failed (non-fatal)`
- Cause: daemon not running or `/api/evolution/proposals` endpoint unavailable.
- Fix: verify daemon is running on M1 Pro: `ssh m1Pro "curl -sf http://localhost:8420/api/health"`.
  If down: `ssh m1Pro "cd ~/GitHub/ConvergioPlatform && ./daemon/start.sh"`.

## Node Deployment (Plan 732)

**deploy-node.sh fails on DB sync**
- Symptom: `scripts/mesh/deploy-node.sh` exits with "rsync: connection unexpectedly closed".
- Cause: SSH alias in the deploy script does not match the entry in `~/.claude/config/peers.conf`.
- Fix: verify the target node name in `peers.conf` matches the SSH alias used by `deploy-node.sh`.
  Run `grep <nodename> ~/.claude/config/peers.conf` and align aliases.

**"keychain User interaction not allowed"**
- Symptom: deploy script fails with "errSecInteractionNotAllowed" when accessing keychain.
- Cause: script is launched from a non-interactive context (launchd, cron, SSH non-interactive).
- Fix: run `scripts/mesh/deploy-node.sh` from an interactive terminal session (Terminal.app or iTerm2),
  not from launchd or a background agent.

**node readiness shows role FAIL**
- Symptom: `GET /api/node/readiness` returns a check with `name: "role"` and `status: "FAIL"`.
- Cause: the node's hostname is not registered in `~/.claude/config/peers.conf` under the correct role.
- Fix: add or update the hostname entry in `peers.conf` with the correct role field, then restart daemon:
  `./daemon/start.sh`

## libSQL Migration (Plan 742)

**Sync columns missing after upgrade**
- Symptom: daemon logs "no such column: updated_at" on sync endpoints.
- Cause: migration did not run (daemon started from old binary).
- Fix: rebuild and restart: `cd daemon && cargo build --release && ./daemon/start.sh`

**Peer resolver returns wrong node**
- Symptom: delegation sends task to unexpected node, or "peer not found" error.
- Cause: `~/.claude/config/peers.conf` entry missing or hostname changed.
- Fix: `grep <expected_name> ~/.claude/config/peers.conf` — add/update entry, restart daemon.
  Resolution order: exact match > prefix match > Tailscale MagicDNS.

**Evidence gate deadlock on shutdown**
- Symptom: daemon hangs on SIGTERM for >10s, lock files left in `/tmp/`.
- Cause: verify mutex held during shutdown race (pre-v19.0.0 bug).
- Fix: upgrade to v19.0.0 (shutdown reaper cleans locks). Manual: `rm /tmp/convergio-verify-*.lock`

**Credential routing fails for multi-org repos**
- Symptom: `gh api` returns 401 for repos in non-default GitHub org.
- Cause: gh credential helper not configured per-repo.
- Fix: run `scripts/platform/gh-credential-route.sh <repo-path>` to configure per-repo credential routing.

**copilot-plan-runner exits without completing all tasks**
- Symptom: runner exits 0 but tasks remain in_progress.
- Cause: pre-v19 runner called `set -e` and exited on first non-zero CLI return.
- Fix: upgrade runner script. v19 removes `set -e`, protects CLI launch, only resets in_progress tasks.

**copilot-plan-runner shows "Next: none" despite pending tasks**
- Symptom: runner logs `Wave: ? | Next: none | Thor: false` even though tasks are pending.
- Cause: execution-context API queries `branch_name` from plans table, but column was missing (added in v19.1.0+).
- Fix: rebuild daemon (`cargo build --release`), restart. Migration auto-adds `branch_name`. Then set it: `curl -X POST localhost:8420/api/plan-db/set-worktree/<plan_id> -H 'Content-Type: application/json' -d '{"worktree_path":"/path","branch_name":"feat/branch"}'`.

## Voice Engine (Plan 748)

**No audio input device found**
- Symptom: `VoiceError::AudioError("no default audio input device")`.
- Cause: no microphone available, or macOS permission not granted.
- Fix: System Settings → Privacy & Security → Microphone → enable for Terminal/IDE.

**Whisper model not found**
- Symptom: `VoiceError::ModelNotAvailable("whisper model not found: ~/.cache/whisper/ggml-small.bin")`.
- Fix: download the model: `curl -L -o ~/.cache/whisper/ggml-small.bin https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin`
- Alt: set `WHISPER_MODEL_PATH=/path/to/model.bin`.

**Voxtral TTS fails with "voxtral_tts not found"**
- Cause: `mlx-audio` installed from PyPI 0.4.1 (lacks voxtral_tts).
- Fix: `pip install git+https://github.com/lucasnewman/mlx-audio.git@main`

**VAD rejects all frames**
- Cause: frame size not 10/20/30ms at 16kHz (160/320/480 samples).
- Fix: ensure `CaptureConfig::frame_duration_ms` is 10, 20, or 30.

**Voice feature not compiling**
- Fix: `cd daemon && cargo build --features voice`. Requires: cpal, webrtc-vad, whisper-rs, hound, ringbuf.

## macOS / Terminal

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
