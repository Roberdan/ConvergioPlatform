<!-- v4.0.0 — Merged: enforcement + workflow-enforced -->
# Enforcement

## Workflow

`GOAL → /solve → /planner (Opus) → DB → /execute {id} → thor/wave (Opus) → merge → done`

> `/prompt` is deprecated — absorbed into `/solve` phase 4 (requirements clarification). Direct `/planner` without `/solve` = BLOCKED (for standard/full triage).

## Step-by-Step (HOOK-ENFORCED — skip ANY = BLOCKED)

| Step | What | How | Blocked |
|---|---|---|---|
| 1 | Triage | `/solve` (full triage + requirements) | Direct /planner without /solve |
| 2 | Plan | `Skill(skill="planner")` **on Opus** | EnterPlanMode |
| 3 | Review | 1x `Agent(plan-reviewer)` → `cvg review register` | Skipping review |
| 4 | DB | `cvg review create` + `cvg plan import` (after review passes) | cvg plan create/import, manual INSERT |
| 5 | Execute | `Skill(skill="execute", args="{id}")` | Direct file edits during plan |
| 6 | Done | `cvg task update {id} done` | cvg task update done (direct, non-safe path) |
| 7 | Thor task | `cvg task validate {id} {plan}` | Wave advance without Thor |
| 8 | Thor wave | `cvg plan validate {wave_id}` | Merge without wave Thor |
| 9 | Merge | `cvg wave merge {plan} {wave}` | Unresolved PR comments |
| 10 | Close | `cvg plan complete {plan_id}` | Pending tasks |
| 11 | Calibrate | `cvg plan calibrate-estimates` (auto after close) | Skipping calibration |
| 12 | Post-mortem | `Agent(plan-post-mortem)` → `cvg plan add-learning` | No learnings extracted |

Single fixes: direct edit OK. Hook only blocks during active plan execution.

## Block Messages

| Violation | Block message |
|---|---|
| Edit/Write during active plan without task-executor | "Use Skill(execute) — direct edits during plans are blocked" |
| `cvg task update done` without running tests | "Show test output before marking done" |
| Declaring "done" without Thor | "Run cvg task validate first" |
| Advancing wave without all tasks Thor-validated | "N tasks still in submitted status" |
| Merging with unresolved PR comments | "Resolve all PR threads first" |
| Skipping checkpoint after task | "Run cvg checkpoint save first" |

## Quick Recovery

| "I forgot to..." | Just run |
|---|---|
| Update DB after task | `cvg task update {id} done "summary"` |
| Run Thor | `cvg task validate {id} {plan}` |
| Checkpoint | `cvg checkpoint save {plan_id}` |
| Resume after compaction | `cvg checkpoint restore {plan_id}` |

## Guardian

Done = tested+committed+evidence. Each F-xx: [x]. User approves closure.

## NON-NEGOTIABLE Rules

**Plan closure**: All PRs MERGED. Worktrees clean. Docs updated. `--force` = user approval.
**Git**: No bare branches. Conventional commits. NEVER `git merge main` → rebase. PR: CI green → fix ALL in one commit → resolve threads → `pr-ops.sh merge`.
**CI batch**: Wait FULL CI → ALL failures → one commit. Max 3 rounds.
**Zero debt**: ALL issues. Touched file = own ALL. _Plan v21._
**Tests**: Update for new behavior. NEVER revert implementation.
**Integration**: New code → wired. Orphan = REJECTED. _Plan 100027._
**Versioning**: fix→patch, feat→minor, breaking→major. CHANGELOG + tag.
**Anti-cheat**: Done without tests/output, defer, suppress, stubs = REJECTION.
**Assessment**: ALL F-xx mapped. Silent exclusion = VIOLATION. _Plan 18.5.0._
**Schema**: Model change → migration same PR. _PR #235._
**Smoke test**: Auth plans → 200 + non-empty. _v19.1.0._
**Cross-plan**: `conflict-check-spec` before parallel. _Plans 383+387._
**Learning**: `session-learnings.sh summary` → Analyze→Propose→Apply→Verify. _Thor 10._
**Verify paths**: New files use glob/find. _Plan 100028._
**Pre-merge**: `pre-merge-gate.sh`. `task-file-tracker.sh`. _Plan v21._
**Compaction**: Self-contained specs. Checkpoint after EVERY task (`cvg checkpoint save`). _Plan 382._
