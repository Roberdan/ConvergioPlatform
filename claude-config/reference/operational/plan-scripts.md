<!-- v5.0.0 -->

# Plan & DB Scripts

`cvg plan` wraps SQLite (`wave_id_fk` numeric FK). NEVER invent columns/subcommands. Schema: `reference/operational/plan-db-schema.md`. NEVER create plans without `/planner` skill. _Why: Plan 225 — bypassing planner skips DB registration, breaking Thor tracking. See `reference/operational/agent-routing.md` § EnterPlanMode._

## Statuses

Task: `pending|in_progress|submitted|done|blocked|skipped|cancelled` | Plan: `todo|doing|done|cancelled` | Wave: `pending|in_progress|done|blocked|merging|cancelled`

Lifecycle: `in_progress` → `submitted` → `done`. `cvg task update` writes `submitted`. Only Thor (`cvg task validate`) → `done`. Trigger `enforce_thor_done` blocks non-Thor `done`.

## Commands

```bash
cvg plan create {proj} "Name" --source-file {f} --auto-worktree --human-summary "..."
cvg plan import {plan_id} spec.yaml
cvg task update {id} done "Summary"
cvg task validate {tid} {plan} | cvg plan validate {wid} | cvg plan validate {id}
cvg plan cancel {plan_id} "reason" | cvg plan cancel-wave {wid} "reason" | cvg plan cancel-task {tid} "reason"
cvg plan execution-tree {plan_id} | cvg plan conflict-check {id} | cvg plan conflict-check-spec {proj} spec.yaml
cvg plan wave-overlap check-spec spec.yaml | cvg plan auto-approve {plan_id} "reason" | cvg plan update-summary {plan_id} "..."
```

## Fix: `complete` Fails

```bash
sqlite3 ~/.claude/data/dashboard.db "SELECT id,task_id,title,status FROM tasks WHERE plan_id={ID} AND status NOT IN ('done','validated','skipped','cancelled');"
cvg task update {DB_ID} done "Reason"
cvg task validate {DB_ID} {PLAN_ID}
cvg plan complete {PLAN_ID}
```

## Cluster & Locking

`cvg plan cluster-status|remote-status|token-report|autosync start|stop|status` | `cvg lock acquire|release|check|list|cleanup <file> [task_id]` | `cvg plan stale-check snapshot|check|diff|merge-queue enqueue|process|status`

## Bootstrap

`cvg plan init` | `service-digest.sh ci|pr|deploy|all` | `worktree-cleanup.sh --all-merged` | `copilot-sync.sh status|sync` | CLI: `cvg plan list [-n]` | DB: `~/.claude/data/dashboard.db` | Sync: `dbsync status|pull|push|incremental`
