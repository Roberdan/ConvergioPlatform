# Execute — Error Handling

## Task Failure

| Error | Action |
|---|---|
| Verify command fails | Fix code, re-run verify, resubmit |
| cargo check fails | Fix compilation, do NOT skip |
| Thor rejects | Read rejection reason, fix, resubmit |
| Script not found | Use full path: `bash claude-config/scripts/<script>` |
| Worktree missing | Run `bash claude-config/scripts/worktree-create.sh <branch>` |
| DB error | Check column names against `reference/operational/plan-db-schema.md` |

## Retry Policy

- Max 3 attempts per task
- After 3 failures: mark task `blocked`, escalate to user
- NEVER suppress errors or skip verify commands

## CI Batch Fix

1. Wait for FULL CI run (not partial)
2. Collect ALL failures
3. Fix ALL in ONE commit
4. Push once
5. Max 3 CI rounds — if still failing after 3, escalate

## Parallel Wave Conflicts

If parallel waves (e.g. W2a, W2b, W2c) modify the same file:
- BLOCK and escalate — do NOT force merge
- User decides which change wins
- This should not happen if plan is well-designed (each wave touches different dirs)
