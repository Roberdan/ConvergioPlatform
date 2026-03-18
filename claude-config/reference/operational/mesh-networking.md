<!-- v3.0.0 -->
# Mesh Networking

Distributed task routing across peers via SSH/Tailscale. Coordinator scores by cost/load/privacy.

## Peers

3-node mesh: `<coordinator>` (coordinator), `<linux-worker>` (worker/linux), `<mac-worker-1>` (worker/macos). Actual hostnames in `~/.claude/config/peers.conf` and `config/local.env`.

Location: `~/.claude/config/peers.conf` (env: `PEERS_CONF`)

Required: `ssh_alias`, `user`, `os` (macos|linux), `role` (coordinator|worker|hybrid)
Optional: `tailscale_ip`, `capabilities` (claude,copilot,ollama,opencode), `status`, `mac_address`, `gh_account`, `default_engine`, `default_model`, `runners`, `runner_paths`

Note: `[mesh]` section is global config (shared_secret), not a peer. `lib/peers.sh` filters sections without `role` field.

### Add Peer

```bash
# 1. peers.conf block (required: ssh_alias, user, os, role)
# 2. mesh-load-query.sh --peer NAME --json
# 3. mesh-env-setup.sh --full (persistent tmux "convergio")
# 4. mesh-auth-sync.sh push --peer NAME
# 5. mesh-claude-login.sh NAME --token sk-ant-oat01-TOKEN
```

tmux aliases: `tlm` (<mac-worker-1>), `tlx` (<linux-worker>), `tl` (local). Auto-attach on SSH via `.zshrc`.

## Performance (v3.0.0 — 13 March 2026)

| Component | Before | After | Change |
|-----------|--------|-------|--------|
| workflow-enforcer.sh | 60-280ms | 10-33ms | Single jq + case/esac, zero grep |
| mesh-health.sh (3 nodes) | ~15s seq | 0.5s parallel | Background SSH + wait |
| mesh-sync-config.sh | ~30s seq | ~10s parallel | Parallel peers + tar pipe batch |
| mesh-sync-all.sh verify | ~15s seq | 0.5s parallel | Parallel SSH verification |
| git-digest.sh | 416ms | 252ms | Single awk, 5s TTL cache |
| peers.sh load | 18ms always | 8ms cached | mtime-based /tmp cache |
| SQLite queries | no indexes | indexed + mmap=256MB | 7 new indexes on plans/kb |

### SSH Multiplexing

Already configured for `<linux-worker-ts>` and `mac-dev-ts` in `~/.ssh/config`:
```
ControlMaster auto
ControlPath ~/.ssh/sockets/%r@%h-%p
ControlPersist 10m
```
Socket dir: `~/.ssh/sockets/`. Eliminates 1-2s TCP+key handshake on subsequent connections.

### SQLite Performance

WAL mode enabled. `db_query()` in `plan-db-core.sh` sets per-connection:
```
PRAGMA cache_size = -8000     (32MB)
PRAGMA temp_store = MEMORY
PRAGMA mmap_size = 268435456  (256MB memory-mapped I/O)
```
Indexes: `tasks` (6), `waves` (5), `plans` (3), `knowledge_base` (4), `agent_activity` (7).

### peers.sh Cache

mtime-based cache in `/tmp/peers-cache-mtime-{mtime}`. Invalidated when `peers.conf` changes. Cache stores all `_PEER_*` variables for instant reload.

## Commands

| Command | Purpose |
|---|---|
| `mesh-sync-all.sh [--dry-run] [--peer] [--phase] [--force]` | Unified 3-phase sync |
| `mesh-sync-config.sh [--dry-run] [--peer]` | Parallel tar-pipe config sync |
| `mesh-health.sh [--peer NAME]` | Parallel health check all nodes |
| `mesh-dispatcher.sh --plan ID [--dry-run] [--force-provider]` | Score+dispatch tasks |
| `mesh-load-query.sh [--json] [--peer]` | CPU load + task state |
| `mesh-heartbeat.sh start\|stop\|status\|daemon` | Liveness daemon (30s) |
| `mesh-auth-sync.sh push\|status [--peer\|--all]` | Credential sync |
| `mesh-claude-login.sh <peer\|--all> --token T \| --status` | Deploy OAuth token |
| `mesh-migrate.sh <plan> <peer> [--dry-run] [--no-launch]` | Live plan migration |
| `mesh-discover.sh [--deep]` | Tailscale peer discovery |

Aliases: `c mesh sync`, `c mesh status`, `c mesh discover`, `c mesh load`

## Sync (3 Phases)

| Phase | What | How |
|---|---|---|
| 1. Config+DB | dotclaude + dashboard.db | git bundle + SCP (bidirectional) |
| 2. Repos | Project repos + .env | `git pull --ff-only` + SCP sync_files |
| 3. Verify | Alignment table | Parallel SSH git log per repo |

repos.conf: `path` (required), `branch`, `gh_account`, `sync_files`. SSH auto-prepends Homebrew PATH.
Phase 1 auto-detects newest peer (bidirectional). Phase 2 connectivity check is parallel. Phase 3 runs all SSH in parallel.

## Cost Routing

| Tier | Bonus | Type |
|---|---|---|
| free | +2 | Ollama |
| zero | +1 | Self-hosted VM |
| premium | +0 | Cloud API |

Scoring: capability +3, privacy +3, CPU≤0 +2, CPU≤1 +1, tasks<max +1. Offline = -99.
Privacy: `privacy_required=true` → only `privacy_safe=true` peers. `--force-provider` overrides.

## Auth (NON-NEGOTIABLE: OAuth Only)

**NEVER ANTHROPIC_API_KEY for Claude Code.** OAuth only (Max subscription). API keys = batch-dispatcher.sh only.

Remote login: `claude setup-token` locally → `mesh-claude-login.sh <peer> --token TOKEN`. Token valid 1 year. Auto-removes stale API keys, deploys to `oauth-token.env` (chmod 600).

Credential sync: Claude=status-check-only | Copilot=`gh auth token` via SSH | OpenCode=SCP config | Ollama=SCP env. SSH: always `-n` flag (stdin safety).

## Live Migration

`mesh-migrate.sh <plan_id> <peer> [--dry-run]` — 5 phases: preflight→rsync→DB migrate→auto-launch→report. Prerequisites: SSH, Claude CLI, tmux, no-sleep. Task status: done=kept, in_progress→pending.

## Troubleshooting

| Symptom | Fix |
|---|---|
| Peer offline | `ssh peer true` — check SSH/Tailscale |
| `gh: not found` via SSH | Scripts auto-prepend PATH; manual: `export PATH="/opt/homebrew/bin:$PATH"` |
| SSH loop 1 peer | All SSH MUST use `-n` flag |
| `claude auth login` hangs SSH | Use `setup-token` + `mesh-claude-login.sh` |
| Bundle `unresolved deltas` | `rsync -az --delete ~/.claude/.git/` |
| Git pull fails on peer | `gh auth switch --user <GH_MICROSOFT_ACCOUNT>` |
| Heartbeat PID stale | `rm ~/.claude/data/mesh-heartbeat.pid && mesh-heartbeat.sh start` |
| `ANTHROPIC_API_KEY` on peer | `mesh-claude-login.sh` auto-removes; check `.zshenv` |
| `lib/lib/peers.sh` error | Fixed v3.0.0: `common.sh` overrides SCRIPT_DIR. Use `_SYNC_SCRIPT_DIR` in callers. |

## Delegation System (v11.6.0)

### Plan-DB HTTP API

Scripts can access plan data via the HTTP API (port 9421) instead of direct sqlite3:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/plan-db/context/:plan_id` | GET | Full plan+waves+tasks for execution |
| `/api/plan-db/json/:plan_id` | GET | Compact plan JSON |
| `/api/plan-db/task/update` | POST | Update task status `{task_id, status, notes, tokens}` |
| `/api/plan-db/agent/start` | POST | Register agent activity `{agent_id, agent_type, ...}` |
| `/api/plan-db/agent/complete` | POST | Mark agent done `{agent_id, tokens_in, ...}` |
| `/api/health` | GET | Health check with `uptime_secs` |

Shell helper: `source scripts/lib/plan-db-http.sh` → `_api_get_context`, `_api_update_task`, etc. Auto-falls back to sqlite3 if daemon is down.

### Delegation

```bash
mesh-delegate-plan.sh <plan_id> <target_peer> [--model m] [--monitor]
```

Verifies peer reachability, syncs DB, checks daemon health, launches copilot-worker remotely.

### Notifications

`mesh-notify.sh <severity> <title> <message>` — dispatches to macOS osascript / Linux notify-send / webhook / DB.

### Quality Gates

`verify-content-quality.sh <path> [--min-loc N] [--strict]` — checks min LOC, stub detection, template detection.
