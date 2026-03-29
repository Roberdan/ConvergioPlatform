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
- Cause: repo behind main. Fix: `ssh m1Pro "cd ~/GitHub/ConvergioPlatform && git pull --rebase origin main"`

**Telegram notification skipped** → add `CONVERGIO_TELEGRAM_TOKEN` + `CONVERGIO_TELEGRAM_CHAT_ID` to `~/.convergio/env`.

**Evolution proposal submission fails (non-fatal)** → daemon not running on M1 Pro. Start it.

## Node Deployment (Plan 732)

**deploy-node.sh "rsync: connection unexpectedly closed"**
- Cause: SSH alias doesn't match `peers.conf`. Fix: align aliases in `~/.claude/config/peers.conf`.

**"keychain User interaction not allowed"**
- Cause: non-interactive context (launchd/cron). Fix: run from interactive terminal.

**node readiness role FAIL**
- Cause: hostname not in `peers.conf` with correct role. Fix: add entry, restart daemon.

## Daemon Sync / Replication (Plan 10004)

**Check sync health**: `curl -s http://localhost:8420/api/sync/status | jq .`
- Returns: `healthy`, `last_success_at`, `last_error`, `transport_mode`, per-peer/per-table breakdown.

**Sync unhealthy** (`"healthy": false`)
- Check `last_error`. Common: peer unreachable, auth failure, DB locked.
- Fix: verify peer (`tailscale ping <peer>`), restart daemon on both nodes.

**Rows not syncing (timestamp mismatch)**
- Cause: `updated_at` format mismatch (SQLite space vs RFC3339 `T`).
- Fix: rebuild daemon (v19.6.0+) — normalizes automatically.

**Two-node verification**: create plan on node A, verify on node B within 60s SLA.
- Harness: `cargo test two_node` in daemon/.

**Transport**: `daemon-http` default. Fallback: `manual-rsync-only` (operator must run `scripts/kernel/sync-db.sh`).

## libSQL Migration (Plan 742)

**Sync columns missing ("no such column: updated_at")**
- Cause: old binary. Fix: `cd daemon && cargo build --release && ./daemon/start.sh`

**Peer resolver returns wrong node**
- Cause: `peers.conf` entry missing/stale. Resolution: exact > prefix > Tailscale MagicDNS.
- Fix: update `~/.claude/config/peers.conf`, restart daemon.

**Evidence gate deadlock on shutdown**
- Cause: pre-v19.0.0 mutex race. Fix: upgrade to v19.0.0. Manual: `rm /tmp/convergio-verify-*.lock`

**Credential routing 401 for multi-org repos**
- Fix: `scripts/platform/gh-credential-route.sh <repo-path>`

**copilot-plan-runner exits without completing all tasks**
- Cause: pre-v19 `set -e`. Fix: upgrade runner script (v19 removes `set -e`).

**copilot-plan-runner "Next: none" despite pending tasks**
- Cause: `branch_name` column missing (pre-v19.1.0). Fix: rebuild daemon, restart.

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
- Cause: hook overhead + inline merge conflict resolution.
- Fix: consolidated hooks (`pre-tool-guard.sh`), delegate cherry-picks, checkpoint after every task. ADR-0113.

**Thor verify "cd daemon: No such file or directory"**
- Cause: daemon cwd already `daemon/`, so `cd daemon &&` resolves to `daemon/daemon/`.
- Fix: remove `cd daemon &&` prefix from verify commands in plan specs.

**plan-checkpoint.sh "Cannot reach daemon"**
- Cause: v1 used wrong API path or `sqlite3` directly.
- Fix: rebuild daemon (v2.0 uses `cvg plan show`).

## Plan Workflow

**"deliverable not found"** → `cvg deliverable list --project <id>`

**workspace create fails** → start daemon, check parent dir permissions.

**Release pipeline stuck** → set `GITHUB_TOKEN`, run quality gate manually first.

**file_sizes gate fails** → split `.rs` files exceeding 250 lines into submodules.

**OpenClaw cannot reach daemon** → `./daemon/start.sh`; `curl http://localhost:8420/api/health`
