# Workflow (ENFORCED BY HOOKS — NOT OPTIONAL)

## The Only Flow That Works

```
GOAL -> /planner (Opus) -> DB approved -> /execute {id} -> thor per-task -> thor per-wave -> merge -> done
```

## Step-by-Step (skip ANY = BLOCKED by hooks)

| Step | What | How | Hook enforces |
|------|------|-----|---------------|
| 1 | Capture goal | `/prompt` or user message | — |
| 2 | Plan | `Skill(skill="planner")` **on Opus** | Blocks `EnterPlanMode` |
| 3 | DB create | `cvg review create` (after reviews) | Blocks `cvg plan create/import` |
| 4 | Execute | `Skill(skill="execute", args="{id}")` | Blocks direct file edits during plan |
| 5 | Task done | `cvg task update {id} done` | Blocks direct task done (non-safe path) |
| 6 | Thor task | `cvg task validate {id} {plan}` | Blocks wave advance without Thor |
| 7 | Thor wave | `cvg plan validate {wave_id}` | Blocks merge without wave Thor |
| 8 | Merge | `cvg wave merge {plan} {wave}` | Blocks with unresolved PR comments |
| 9 | Repeat 4-8 | Next wave | — |
| 10 | Close | `cvg plan complete {plan_id}` | Blocks if tasks not validated |

## What Gets You BLOCKED

| Violation | Block message |
|-----------|--------------|
| Edit/Write during active plan without task-executor | "Use Skill(execute) — direct edits during plans are blocked" |
| `cvg task update done` without running tests | "Show test output before marking done" |
| Declaring "done" without Thor | "Run cvg task validate first" |
| Advancing wave without all tasks Thor-validated | "N tasks still in submitted status" |
| Merging with unresolved PR comments | "Resolve all PR threads first" |
| Skipping checkpoint after task | "Run cvg checkpoint save first" |

## Single Fix for Each Problem

| "I forgot to..." | Just run |
|-------------------|----------|
| Update DB after task | `cvg task update {id} done "summary"` |
| Run Thor | `cvg task validate {id} {plan}` |
| Checkpoint | `cvg checkpoint save {plan_id}` |
| Resume after compaction | `cvg checkpoint restore {plan_id}` |

## For Single Fixes (No Plan Needed)

Direct edit is fine for isolated fixes. The hook only blocks during active plan execution.
