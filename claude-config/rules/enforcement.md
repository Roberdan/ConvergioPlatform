# Enforcement

## Workflow

`/solve → /planner (Opus) → review → DB → /execute (Codex) → thor (Opus) → merge → done`

`/prompt` deprecated. Direct `/planner` without `/solve` = BLOCKED.

## Steps (HOOK-ENFORCED — skip = BLOCKED)

| # | What | How |
|---|---|---|
| 1 | Triage | `/solve` |
| 2 | Plan | `Skill(skill="planner")` Opus |
| 3 | Review | `Agent(plan-reviewer)` → `cvg review register` |
| 4 | DB | `cvg review create` + `cvg plan import` |
| 5 | Execute | `Skill(skill="execute", args="{id}")` |
| 6 | Done | `cvg task update {id} done` |
| 7 | Thor task | `cvg task validate {id} {plan}` |
| 8 | Thor wave | `cvg plan validate {wave_id}` |
| 9 | Merge | `cvg wave merge {plan} {wave}` |
| 10 | Close | `cvg plan complete {plan_id}` |
| 11 | Calibrate | `cvg plan calibrate-estimates` |
| 12 | Post-mortem | `Agent(plan-post-mortem)` → `cvg plan add-learning` |

Single fixes: direct edit OK.

## Blocks

Edit/Write during plan → `Skill(execute)` | Done without tests → show output | Done without Thor → `cvg task validate` | Merge with open PR comments → resolve | Skip checkpoint → `cvg checkpoint save`

## Recovery

DB → `cvg task update {id} done` | Thor → `cvg task validate {id} {plan}` | Checkpoint → `cvg checkpoint save` | Compaction → `cvg checkpoint restore`

## NON-NEGOTIABLE Rules

**Plan closure**: All PRs MERGED. Worktrees clean. Docs updated.
**Git**: Conventional commits. NEVER `git merge main` → rebase. CI green → fix ALL → resolve threads.
**CI batch**: Full CI → ALL failures → one commit. Max 3 rounds.
**Versioning**: fix→patch, feat→minor, breaking→major. CHANGELOG + tag.
**Anti-cheat**: Done without tests/output, defer, suppress, stubs = REJECTION.
**Assessment**: ALL F-xx mapped. Silent exclusion = VIOLATION.
**Smoke test**: Auth plans → 200 + non-empty.
**Cross-plan**: `conflict-check-spec` before parallel.
**Learning**: `session-learnings.sh summary` → Analyze→Propose→Apply→Verify.
**Verify paths**: New files use glob/find.
