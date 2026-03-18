# Compaction Preservation

Rewriting ANY file: these MUST survive. Removing = VIOLATION.

## NEVER Remove

| Category | Examples |
|---|---|
| Quality gates | `test:unit`, `ci:summary`, `i18n:check`, `prisma generate` |
| Thor validation | `validate-wave` (Opus, wave-only), quality gates |
| Pre-commit hooks | i18n:check, smart-test, env-var-audit |
| Verify steps | "run tests before commit", "i18n sync after UI", "env var checklist" |
| Security | CSP, RBAC, parameterized queries, WCAG, encryption |
| State mgmt | "NO localStorage", "Zustand + REST only", session-based auth |
| Worktree | "NEVER git checkout on main", worktree-create.sh |
| Routing | `/planner` for plans, `/execute` for tasks, EnterPlanMode = VIOLATION |
| Docs | TROUBLESHOOTING.md, per-wave ADR, gate 9b, problem-resolution order |
| Learning | Thor 10, Analyze→Propose→Apply→Verify, `_Why:` annotations |

Checklist: (1) Diff old vs new (2) No removal from above (3) All CLI commands preserved (4) All workflow steps preserved

SAFE: Remove prose, tables, abbreviate | FORBIDDEN: Remove workflow steps, drop verify commands, merge gates, remove `_Why:`
