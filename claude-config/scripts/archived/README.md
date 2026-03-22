# Archived Scripts

**Archived**: 2026-03-22
**Reason**: Plan 685 — Migration to `cvg` CLI (Rust). All SQLite shell wrappers and plan-db orchestration scripts replaced by the unified `cvg` binary. See ADR in docs/adr/ for plan 685 decisions.

**Retention**: 30 days from archive date (delete after 2026-04-21)

## Scripts Archived

### Core plan-db wrappers (replaced by `cvg` commands)
- `plan-db.sh` — Main plan database CLI wrapper
- `plan-db-safe.sh` — Safe task update wrapper (done status)
- `plan-db-verify.sh` — DB verification utility
- `plan-db-autosync.sh` — Automatic DB sync script
- `planner-create.sh` — Plan creation workflow script

### lib/plan-db-*.sh (SQLite shell modules — all replaced by `cvg`)
- `plan-db-agents.sh` — Agent tracking queries
- `plan-db-cluster.sh` — Cluster management queries
- `plan-db-conflicts.sh` — Conflict detection queries
- `plan-db-core.sh` — Core DB operations
- `plan-db-create.sh` — Plan/task creation queries
- `plan-db-crud.sh` — Generic CRUD operations
- `plan-db-delegate.sh` — Delegation management queries
- `plan-db-delete.sh` — Deletion operations
- `plan-db-display.sh` — Display/formatting utilities
- `plan-db-drift.sh` — Drift detection queries
- `plan-db-http.sh` — HTTP/API DB operations
- `plan-db-import.sh` — Plan import logic
- `plan-db-intelligence.sh` — Intelligence/analytics queries
- `plan-db-knowledge.sh` — Knowledge base queries
- `plan-db-read.sh` — Read/query operations
- `plan-db-remote.sh` — Remote sync operations
- `plan-db-update.sh` — Update operations
- `plan-db-validate.sh` — Validation queries

### Infrastructure scripts (replaced by `cvg` or mesh daemon)
- `session-reaper.sh` — Session cleanup utility
- `project-audit.sh` — Project audit script

## Preserved (NOT archived)

The following were explicitly kept:
- All `*-digest.sh` scripts (ci-digest.sh, git-digest.sh, etc.) — still active
- `worktree-create.sh` — Used by `cvg wave create-worktree`
- `wave-worktree.sh` — Used by merge workflow
