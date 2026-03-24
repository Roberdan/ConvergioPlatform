# Compaction Preservation

Rewriting ANY file: these MUST survive. Removing = VIOLATION.

## NEVER Remove

Quality gates (`test:unit`, `ci:summary`) | Thor validation | Pre-commit hooks | Verify steps | Security (CSP, RBAC, parameterized, WCAG) | Worktree discipline | Routing (`/planner`, `/execute`) | Docs (TROUBLESHOOTING, ADR, gate 9b) | Learning (Thor 10, `_Why:`)

## Checklist

(1) Diff old vs new (2) No removal from above (3) CLI commands preserved (4) Workflow steps preserved

SAFE: prose, tables, abbreviate | FORBIDDEN: workflow steps, verify commands, gates, `_Why:`
