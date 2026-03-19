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

Executors CANNOT set status=done. SQLite trigger `enforce_thor_done` blocks it. Only `plan-db.sh validate-wave` (called by @validate at wave level) can batch-promote submitted → done.

## Routing Rules
- Read `executor_agent` from DB per task.
- Default route is `copilot`.
- Use `claude` only when explicitly assigned.
- Always pass worktree path, constraints, readiness bundle, and CI knowledge.

## Required Flow
1. Initialize + auto-heal plan/worktree metadata.
2. Run readiness checks and stop on critical warnings.
3. Run drift check (MANDATORY before first task).
4. Dispatch pending tasks via selected executor.
5. Track agent lifecycle + task substatus transitions.
6. Per-wave Thor validation (NOT per-task).
7. Apply wave merge mode (`sync`/`batch`/`none`).
8. Validate and complete plan in DB.

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
