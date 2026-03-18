<!-- v5.0.0 -->

# Worktree Discipline

## No Bare Branches (NON-NEGOTIABLE)

NEVER `git branch`, `git checkout -b`, `git switch -c`. Hooks: `worktree-guard.sh`, `enforce-worktree-boundary.sh`.

| Need | Use | NEVER |
|------|-----|-------|
| Plan work | `wave-worktree.sh create <plan> <wave>` | `git checkout -b plan/xxx` |
| Feature branch | `worktree-create.sh <branch> [path]` | `git branch feature/xxx` |
| Task isolation | `Task(..., isolation="worktree")` | `git checkout -b task-xxx` |
| Quick fix on main | Direct edit (no branch) | `git checkout -b fix/xxx` |

## Wave-per-Worktree v2

Lifecycle: `create → execute → Thor → rebase (update FROM main) → PR → squash merge (INTO main) → cleanup`

`wave-worktree.sh create|merge|cleanup|status <plan_id> <wave_db_id>`. DB: `plan-db.sh get-wave-worktree|set-wave-worktree <id> [path]`.

Wave status: `pending` → `in_progress` → `merging` → `done`. Branch: `plan/{plan_id}-{wave_id}`. DB columns: `worktree_path|branch_name|pr_number|pr_url|merge_mode|theme`.

### Git Graph Hygiene (NON-NEGOTIABLE)

NEVER `git merge main` into wave branch. To update FROM main: `git rebase origin/main`. To merge PR INTO main: `wave-worktree.sh merge` (squash merge). `wave-worktree.sh merge` auto-rebases before squash.

## Subagent Worktree Isolation

`Task(subagent_type="task-executor", isolation="worktree")` — auto temp worktree. WorktreeCreate: symlink `.env*`, `npm install`. WorktreeRemove: cleanup, release locks. Complementary to wave-worktree.

Legacy v1: `reference/archive/worktree-v1.md`. node_modules: `npm ci` (symlinks fail with Turbopack/Sentry).
