# Execute — Error Handling

## Task Failure

| Error | Action |
|---|---|
| Verify command fails | Fix code, re-run verify, resubmit |
| cargo check fails | Fix compilation, do NOT skip |
| Thor rejects | Read rejection reason, fix, resubmit |
| Script not found | Use full path: `bash claude-config/scripts/<script>` |
| Worktree missing | Run `cvg workspace create-feature <branch>` |
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

## Infrastructure Gate Retry Limit

- Se un gate (evidence, kernel, Thor) fallisce **2 volte consecutive**, l'executor DEVE:
  1. Fermarsi
  2. Riportare all'utente il messaggio di errore esatto
  3. Chiedere se procedere o risolvere manualmente
- **MAI debuggare infrastruttura daemon** (porte, auth token, endpoint mismatch, schema DB) nella stessa sessione di esecuzione piano. Se il daemon non risponde o risponde con errori di schema, FERMARSI e segnalare.
- _Why: sessione 10040 — 15+ shell bruciate su retry evidence gate perché il worktree_path era null e il kernel eseguiva cargo_test su un progetto Next.js._
