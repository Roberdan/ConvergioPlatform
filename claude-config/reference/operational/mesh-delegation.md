# Mesh Delegation

End-to-end guide for delegating plan execution to remote mesh peers.

## Overview

```
Coordinator (M5Max) ──rsync──> Worker (M1 Pro)
                                 └─ tmux session
                                 └─ claude CLI (autonomous)
                                 └─ cvg task update → submitted
                                 └─ Thor validates → done
                                 └─ git push → PR
```

## Prerequisites

| Requirement | Check |
|-------------|-------|
| Peer in peers.conf | `cvg mesh status` |
| SSH connectivity | `ssh <peer> true` |
| Daemon running on peer | `ssh <peer> curl -sf localhost:8420/api/health` |
| Claude CLI authenticated | `ssh <peer> claude --version` |
| DB synced | `scripts/kernel/sync-db.sh` |

## Step-by-Step Delegation

### 1. Sync DB to Peer

```bash
# rsync-based sync (filesystem, NOT git)
scripts/kernel/sync-db.sh $(hostname) <peer>
```

### 2. Start Delegation

```bash
# Via cvg CLI
cvg delegation start <plan_id> <peer_name>

# Or via mesh script
mesh-delegate-plan.sh <plan_id> <peer> [--model claude-opus-4.6] [--monitor]
```

### 3. Worker Lifecycle (on peer)

The worker follows the NON-NEGOTIABLE lifecycle from delegation-protocol.md:

```bash
# 1. Register
cvg agent start "claude-$(hostname -s)" --task-id <first_task_db_id>

# 2. Per task
cvg task update <db_id> in_progress
# ... do work (TDD, verify, cargo check/test) ...
cvg task update <db_id> submitted --summary "what was done"

# 3. Per wave
cvg plan validate <plan_id>
git add -A && git commit -m "feat(plan-<id>): wave summary"

# 4. Complete
git push origin <branch>
cvg agent complete "claude-$(hostname -s)"
```

### 4. Monitor Progress

```bash
cvg delegation status <plan_id>    # progress table
cvg plan tree <plan_id> --human    # visual tree
cvg who agents                     # active agents
```

### 5. Post-Delegation

```bash
# Sync DB back
scripts/kernel/sync-db.sh <peer> $(hostname)

# Verify completion
cvg plan tree <plan_id> --human
```

## Peer Resolution

All peer names resolve through `peer_resolver` (daemon/src/mesh/peer_resolver.rs):

| Input | Resolution Strategy |
|-------|-------------------|
| Exact section name | Direct lookup in peers.conf |
| Case-insensitive | `M1Pro` matches `m1pro` |
| SSH alias | Matches `ssh_alias` field |
| Tailscale IP | Matches `tailscale_ip` field |
| DNS name | Substring match on `dns_name` |

See `convergio-decision-tree.md` for "which delegation method to use when".

## Copilot Delegation (Alternative)

For Copilot-based workers (not Claude CLI):

```bash
copilot-worker.sh <task_db_id> --model claude-opus-4.6
copilot-plan-runner.sh <plan_id>   # auto-restart loop
```

NEVER delegate via GitHub Issues — use Convergio scripts only.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| Worker can't reach daemon | Check `DASHBOARD_DB` env var on peer |
| Task stuck in_progress | `cvg task update <id> pending` to reset |
| DB out of sync | `scripts/kernel/sync-db.sh` both directions |
| Auth expired on peer | `mesh-claude-login.sh <peer> --token <TOKEN>` |
| tmux session lost | `ssh <peer> tmux ls` then reattach |
