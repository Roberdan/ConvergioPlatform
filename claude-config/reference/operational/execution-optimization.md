<!-- v4.0.0 -->

# Execution Optimization

## Isolation & Models

Subagents (fresh session): `task-executor` (NO parent context, TDD, no WebSearch/WebFetch) | `thor` (skeptical, unbiased). ALWAYS `task-executor` for plan tasks, NEVER `general-purpose`. Token tracking: `--tokens {N}` on update-task.

| Agent | Default | Escalate |
|-------|---------|----------|
| Task Executor | GPT-5.3-Codex | claude-opus-4.6 (cross-cutting) |
| Coordinator | claude-sonnet-4.6 | claude-opus-4.6 (>3 concurrent) |
| Coordinator (Max Parallel) | **claude-opus-4.6** | required |
| Thor (wave only) | claude-opus-4.6 | — |

## Parallelization

Standard: 3 (claude-sonnet-4.6) | Max Parallel: 5 hard cap (**claude-opus-4.6**) | Agent Teams: native (claude-sonnet-4.6). 36GB RAM limit. Pre-spawn: `session-reaper.sh --pre-spawn`.

## Lean Coordinator

Dispatch + DB + checkpoint ONLY. NEVER Read project files, NEVER read transcripts, NEVER run tests. Max 4 tasks/wave. Checkpoint: after EVERY task → `cvg checkpoint save <plan_id>`. DB update SAME MESSAGE as executor result. _Why: Plan 382 — coordinator context bloat caused missed tasks and stale DB state._

## Post-Task Protocol (MANDATORY)

Per task: (0) checkpoint (1) verify DB — NO per-task Thor (mechanical gates suffice)

Per wave: (2) Thor `cvg plan validate` **(Opus)** (3) `cvg wave merge` (4) PR comments → `Task(subagent_type='pr-comment-resolver')` (5) cleanup: `session-reaper.sh --max-age 0` + verify worktree/branch deleted + wave=done (6) next wave

## Sync & Closure

DB sync: `scp ~/.claude/data/dashboard.db "$host":~/.claude/data/dashboard.db` after updates. _Why: Plan 387 — concurrent runners on `<mac-worker-1>` caused EEXIST errors until npm caches were isolated per runner._ CI: `ci-watch.sh <branch> --repo owner/repo [--sha SHA] [--timeout SEC]`

## Plan Closure (NON-NEGOTIABLE)

`session-reaper.sh --max-age 0` | `git worktree list` (only main) | `git branch | grep plan/` (none) | all PRs merged/closed. Leaving artifacts = VIOLATION.
