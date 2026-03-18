<!-- v5.0.0 -->

# Plan & DB Scripts

`plan-db.sh` wraps SQLite (`wave_id_fk` numeric FK). NEVER invent columns/subcommands. Schema: `reference/operational/plan-db-schema.md`. NEVER create plans without `/planner` skill. _Why: Plan 225 — bypassing planner skips DB registration, breaking Thor tracking. See `reference/operational/agent-routing.md` § EnterPlanMode._

## Statuses

Task: `pending|in_progress|submitted|done|blocked|skipped|cancelled` | Plan: `todo|doing|done|cancelled` | Wave: `pending|in_progress|done|blocked|merging|cancelled`

Lifecycle: `in_progress` → `submitted` → `done`. `plan-db-safe.sh` writes `submitted`. Only Thor (`validate-task`) → `done`. Trigger `enforce_thor_done` blocks non-Thor `done`.

## Commands

```bash
plan-db.sh create {proj} "Name" --source-file {f} --auto-worktree --human-summary "..."
plan-db.sh import {plan_id} spec.yaml
plan-db-safe.sh update-task {id} done "Summary"
plan-db.sh validate-task {tid} {plan} | validate-wave {wid} | validate {id}
plan-db.sh cancel {plan_id} "reason" | cancel-wave {wid} "reason" | cancel-task {tid} "reason"
plan-db.sh execution-tree {plan_id} | conflict-check {id} | conflict-check-spec {proj} spec.yaml
plan-db.sh wave-overlap check-spec spec.yaml | auto-approve {plan_id} "reason" | update-summary {plan_id} "..."
```

## Fix: `complete` Fails

```bash
sqlite3 ~/.claude/data/dashboard.db "SELECT id,task_id,title,status FROM tasks WHERE plan_id={ID} AND status NOT IN ('done','validated','skipped','cancelled');"
plan-db-safe.sh update-task {DB_ID} done "Reason"
plan-db.sh validate-task {DB_ID} {PLAN_ID}
plan-db.sh complete {PLAN_ID}
```

## Cluster & Locking

`plan-db.sh cluster-status|remote-status|token-report|autosync start|stop|status` | `file-lock.sh acquire|release|check|list|cleanup <file> [task_id]` | `plan-db.sh stale-check snapshot|check|diff|merge-queue enqueue|process|status`

## Bootstrap

`planner-init.sh` | `service-digest.sh ci|pr|deploy|all` | `worktree-cleanup.sh --all-merged` | `copilot-sync.sh status|sync` | CLI: `piani [-n]` | DB: `~/.claude/data/dashboard.db` | Sync: `dbsync status|pull|push|incremental`
