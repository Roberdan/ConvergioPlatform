# Execute — Initialize & Readiness

## Initialization

1. Read plan from DB: `bash claude-config/scripts/plan-db.sh execution-tree {plan_id}`
2. Verify plan status = `doing` (if `todo`, run `bash claude-config/scripts/plan-db.sh start {plan_id}`)
3. Read worktree path from plan or W1 wave: `SELECT worktree_path FROM plans WHERE id = {plan_id}`
4. If no worktree: `bash claude-config/scripts/wave-worktree.sh create {plan_id} {wave_id}` or BLOCK

## Script Paths (CRITICAL)

All scripts are in `claude-config/scripts/`. They are NOT in PATH. Always use full path:
```bash
bash claude-config/scripts/plan-db.sh <command>
bash claude-config/scripts/plan-db-safe.sh <command>
bash claude-config/scripts/wave-worktree.sh <command>
bash claude-config/scripts/validate-wave.sh <command>
```
_Why: Plan 677 — executor failed with `command not found: plan-db.sh` in new session._

## Readiness Check

Run `bash claude-config/scripts/planner-create.sh readiness {plan_id}` which checks:

| Check | Requirement |
|---|---|
| Plan status | Must be `doing` |
| Worktree | Must exist on disk |
| Review | At least 1 in `plan_reviews` table |
| test_criteria | Every task must have non-empty `verify[]` array |
| Task count | plan.tasks_total matches actual task count |
| Effort levels | All 1-3 (DB CHECK constraint) |

If readiness passes → proceed. If fails → fix errors, do NOT proceed.

## Drift Check + Rebase (MANDATORY before first task)

Run: `bash claude-config/scripts/plan-db.sh drift-check {plan_id}`

If drift detected (main has new commits since worktree creation):
```bash
cd {worktree_path} && git rebase origin/main
```
This is NORMAL — the planner may commit config changes on main after creating the worktree.
Rebase, then proceed. Do NOT abort because of drift.

_Why: Plan 677 — worktree created, then 7 commits on main (planner/executor fixes).
Drift check correctly detected overlap but executor treated it as blocking error._

## Auto-Heal

If worktree missing but plan is `doing`:
1. Try `bash claude-config/scripts/worktree-create.sh plan-{id}-w1`
2. Update `waves.worktree_path` and `plans.worktree_path` in DB
3. Re-run readiness

If plan is `todo`:
1. `bash claude-config/scripts/plan-db.sh start {plan_id}`
2. Proceed with initialization
