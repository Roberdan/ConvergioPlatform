# Core Workflow

## Plan & DB

`cvg plan` wraps SQLite. Schema: `reference/operational/plan-db-schema.md`
NEVER create plans without `/planner`. _Why: Plan 225._

States — Task: `pending|in_progress|submitted|done|blocked|skipped|cancelled` | Plan: `todo|doing|done|cancelled` | Wave: `pending|in_progress|done|blocked|merging|cancelled`
Lifecycle: `in_progress` → `submitted` → `done` (wave Thor). Thor validates at wave level (Opus).

| Action | Command |
|---|---|
| Create | `cvg plan create {proj} "Name" --source-file {f} --auto-worktree` |
| Import | `cvg plan import {plan_id} spec.yaml` |
| Done | `cvg task update {id} done "Summary"` |
| Validate | `cvg task validate {tid} {plan}` |
| Cancel | `cvg plan cancel {plan_id} "reason"` |
| Debug | `cvg plan execution-tree {plan_id}` |

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
| Plan work | `cvg wave create <plan> <wave>` |
| Feature branch | `worktree-create.sh <branch> [path]` |
| Task isolation | `Task(..., isolation="worktree")` |
| Quick fix | Direct edit (no branch) |

Wave: create → execute → Thor → `git rebase origin/main` → PR → squash merge → cleanup. NEVER `git merge main`.

## Execution

`task-executor` for plan tasks (TDD) | `thor` for validation | Sonnet default | Max 3 parallel | Max 4 tasks/wave | Checkpoint after EVERY task

Post-task: checkpoint → verify DB. Per wave: Thor → `cvg wave merge` → PR comments → cleanup → next wave.

Closure: `session-reaper.sh --max-age 0` | `git worktree list` (only main) | all PRs merged
