<!-- v1.0.0 — Merged: plan-scripts + digest-scripts + worktree-discipline + execution-optimization -->

# Core Workflow

## Plan & DB

- `plan-db.sh` wraps SQLite. Schema: `reference/operational/plan-db-schema.md`
- NEVER create plans without `/planner` skill. _Why: Plan 225 — bypassing planner skips DB registration, breaking Thor tracking._
- Task: `pending|in_progress|submitted|done|blocked|skipped|cancelled`
- Plan: `todo|doing|done|cancelled` | Wave: `pending|in_progress|done|blocked|merging|cancelled`
- Lifecycle: `in_progress` → `submitted` (plan-db-safe.sh) → `done` (wave Thor or plan-db-safe.sh)
- Thor validates at wave level (Opus), not per-task

| Action | Command |
|---|---|
| Create plan | `plan-db.sh create {proj} "Name" --source-file {f} --auto-worktree` |
| Import spec | `plan-db.sh import {plan_id} spec.yaml` |
| Mark done | `plan-db-safe.sh update-task {id} done "Summary"` |
| Validate | `plan-db.sh validate-task {tid} {plan}` |
| Cancel | `plan-db.sh cancel {plan_id} "reason"` |
| Debug | `plan-db.sh execution-tree {plan_id}` |

## Digest Scripts (NON-NEGOTIABLE)

| Instead of | Use |
|---|---|
| `gh run view --log-failed` | `service-digest.sh ci` |
| `gh pr view --comments` | `service-digest.sh pr` |
| `npm install/ci` | `npm-digest.sh install` |
| `npm run build` | `build-digest.sh` |
| `npx vitest`/`npm test` | `test-digest.sh` |
| `git diff main...feat` | `diff-digest.sh main feat` |
| `git status/log` | `git-digest.sh [--full]` |
| `gh pr checks` | `ci-digest.sh checks <pr>` |
| CI polling | `ci-watch.sh <branch> --repo owner/repo` |
| audit/linters | `project-audit.sh --project-root $(pwd)` |

Options: `--no-cache` | `--compact`

## Worktree (NON-NEGOTIABLE)

NEVER `git branch` | `git checkout -b` | `git switch -c`

| Need | Command |
|---|---|
| Plan work | `wave-worktree.sh create <plan> <wave>` |
| Feature branch | `worktree-create.sh <branch> [path]` |
| Task isolation | `Task(..., isolation="worktree")` |
| Quick fix | Direct edit (no branch) |

- Wave: create → execute → Thor → rebase (update FROM main) → PR → squash merge (INTO main) → cleanup
- NEVER `git merge main` into wave branch → use `git rebase origin/main` to update FROM main
- PR merge back to main uses **squash merge** via `wave-worktree.sh merge`

## Execution

- ALWAYS `task-executor` for plan tasks (TDD, no WebSearch)
- `thor` for validation (skeptical, unbiased)
- Default model: Sonnet | Parallelization: max 3
- Lean Coordinator: dispatch + DB + checkpoint ONLY. Max 4 tasks/wave
- Checkpoint after EVERY task

### Post-Task (MANDATORY)

1. Checkpoint → verify DB (no per-task Thor — mechanical gates suffice)
2. Per wave: Thor validate-wave (Opus) → merge → PR comments → cleanup → next wave

### Closure (NON-NEGOTIABLE)

`session-reaper.sh --max-age 0` | `git worktree list` (only main) | all PRs merged
