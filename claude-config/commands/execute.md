---
name: execute
version: "2.3.0"
---

<!-- v2.3.0 -->

# Plan Executor (Compact)

Automated task execution with per-task routing (`copilot` default, `claude` by escalation).

## Activation
`/execute {plan_id}` or `/execute` (current) | Override: `--force-engine claude|copilot`

## CRITICAL: Status Flow (NON-NEGOTIABLE)

```
pending → in_progress → submitted (executor) → done (ONLY Thor)
                              ↓ Thor rejects
                         in_progress (fix and resubmit)
```

Executors CANNOT set status=done. SQLite trigger `enforce_thor_done` blocks it. Only `cvg plan validate-wave` (called by @validate at wave level) can batch-promote submitted → done.

## Routing Rules
- Read `executor_agent` from DB per task.
- Default route is `copilot`.
- Use `claude` only when explicitly assigned.
- Always pass worktree path, constraints, readiness bundle, and CI knowledge.

## Required Flow
1. Initialize: `cvg plan get-context {plan_id}` — returns full JSON with tasks, worktree, constraints.
   - To view tree: `cvg plan show {plan_id}` (alias for `execution-tree`)
   - To view single task: `cvg plan task-detail {plan_id} {task_id}`
   - Auto-heal plan/worktree metadata if needed.
2. Run readiness checks and stop on critical warnings.
3. Run drift check (MANDATORY before first task).
4. **Per-wave loop** (repeat for each wave):
   a. Dispatch pending tasks via selected executor.
   b. Wait for ALL tasks in wave to reach `submitted`.
   c. **MANDATORY Thor gate**: `cvg plan validate-wave {wave_db_id}` — promotes submitted→done, closes wave. NEVER skip. NEVER proceed to next wave without this.
   d. Apply wave merge mode (`sync`/`batch`/`none`).
   e. Output: `--- Wave WX --- Thor: PASS`
5. After ALL waves done: validate and complete plan in DB.

## CRITICAL: CLI

Use `cvg` CLI for all plan/wave/task operations. Examples: `cvg plan get-context {id}`, `cvg wave merge`, `cvg task update`.
_Why: Plan 677 — `command not found` in new session. cvg is in PATH after bootstrap._

## Module References
- Init + readiness: `@reference/commands/execute/initialize-and-readiness.md`
- Task routing + tracking: `@reference/commands/execute/task-routing-and-tracking.md`
- Validation + merge + completion: `@reference/commands/execute/validation-merge-completion.md`
- Error handling: `@commands/execute-modules/error-handling.md`

## Per-Task Mechanical Gates (before submit)

| Check | How |
|---|---|
| Files exist | `test -f` for each artifact |
| Verify commands | Run ALL from `test_criteria.verify[]` |
| Tests pass | Language-appropriate test runner |
| Typecheck | Language-appropriate type checker |
| Line limits | `wc -l < file` (max 250) |

## CI Batch Fix (NON-NEGOTIABLE)

Wait for FULL CI. Collect ALL failures. Fix ALL in one commit. Push once. Max 3 rounds.

## Output Format
`[N/total] task_id: title -> DONE` | `--- Wave WX --- Thor: PASS` | `=== COMPLETE ===`
