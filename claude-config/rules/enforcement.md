<!-- v3.1.0 -->
# Enforcement

## Workflow

`GOAL → /solve → /planner (Opus) → DB → /execute {id} → thor/wave (Opus) → merge → done`

> `/prompt` is deprecated — absorbed into `/solve` phase 4 (requirements clarification). Direct `/planner` without `/solve` = BLOCKED (for standard/full triage).

| Step | How | Blocked |
|---|---|---|
| Triage | `/solve` (full triage + requirements) | Direct /planner without /solve |
| Plan | `Skill(skill="planner")` | EnterPlanMode |
| Review | 1x `Agent(plan-reviewer)` → `cvg review register` | Skipping review |
| DB | `cvg review create` + `cvg plan import` (after review passes) | cvg plan create/import, manual INSERT |
| Execute | `Skill(skill="execute")` | Direct edits |
| Done | `cvg task update {id} done` | cvg task update done (direct) |
| Thor | `cvg plan validate` (Opus, per-wave only) | Advance/merge without Thor |
| Merge | `cvg wave merge` | Unresolved PR comments |
| Close | `cvg plan complete` | Pending tasks |
| Calibrate | `cvg plan calibrate-estimates` (auto after close) | Skipping calibration |
| Post-mortem | `Agent(plan-post-mortem)` → `cvg plan add-learning` | No learnings extracted |

Single fixes: direct edit OK.

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
