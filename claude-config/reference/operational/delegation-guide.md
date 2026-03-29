# Delegation Guide

Complete end-to-end flow: rsync → tmux → claude → Thor → completion.

## Flow Diagram

```
1. PREPARE    Coordinator builds prompt + syncs DB
2. CONNECT    SSH to peer, create tmux session
3. EXECUTE    Claude CLI runs autonomously (TDD per task)
4. VALIDATE   Thor validates each wave
5. COMPLETE   Push, sync back, close delegation
```

## Phase 1: Prepare

```bash
# Sync DB to peer (rsync-based, NOT git)
scripts/kernel/sync-db.sh $(hostname) <peer>

# Build delegation prompt (auto-includes task context)
copilot-task-prompt.sh <task_db_id> [role]
```

## Phase 2: Connect

```bash
# Start delegation (creates tmux session on peer)
cvg delegation start <plan_id> <peer>

# Manual equivalent:
ssh <peer> tmux new-session -d -s "plan-<id>" \
  "cd /path/to/repo && claude --prompt-file /tmp/plan-<id>.md"
```

## Phase 3: Execute (on peer)

The worker follows the lifecycle in `delegation-protocol.md`:

```bash
# Register agent
cvg agent start "claude-$(hostname -s)" --task-id <first_task>

# Per task (TDD: RED → GREEN → verify)
cvg task update <db_id> in_progress
# 1. Write failing test
# 2. Implement until test passes
# 3. cargo check && cargo test
cvg task update <db_id> submitted --summary "..."

# Per wave
cvg plan validate <plan_id>
git add -A && git commit -m "feat(plan-<id>): <wave> summary"
```

## Phase 4: Validate

Thor runs automatically on `cvg plan validate`:

| Gate | Check |
|------|-------|
| 1 | All tasks submitted |
| 2 | Tests pass |
| 3 | Build clean |
| 4 | No lint errors |
| 5 | Files exist (verify commands) |
| 6-10 | Coverage, mocks, evidence, docs, anti-cheat |

Tasks promoted: `submitted` → `done` only by Thor.

## Phase 5: Complete

```bash
# On peer
git push origin <branch>
cvg agent complete "claude-$(hostname -s)"

# On coordinator
scripts/kernel/sync-db.sh <peer> $(hostname)   # sync DB back
cvg plan tree <plan_id> --human                 # verify
cvg delegation status <plan_id>                 # final status
```

## Monitoring During Execution

```bash
cvg delegation status <plan_id>    # progress table
cvg who agents                     # active agents
cvg plan tree <plan_id> --human    # visual tree
ssh <peer> tmux attach -t plan-<id>  # watch live
```

## Alternative: Copilot Workers

```bash
# Single task
copilot-worker.sh <task_db_id> --model claude-opus-4.6

# Full plan (auto-restart loop)
copilot-plan-runner.sh <plan_id>
```

Scripts handle: agent tracking, TDD prompts, retries, mesh events, DB updates.

## Key Rules (NON-NEGOTIABLE)

- Sync via **rsync** (filesystem), NEVER git for DB sync
- Workers set tasks to **submitted**, NEVER done — Thor promotes
- **TDD mandatory**: RED → GREEN → evidence
- NEVER delegate via GitHub Issues
- Agent identity required: `cvg agent start/complete`
