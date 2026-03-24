# Code Standards

## Style

| Lang | Standard |
|---|---|
| TS/JS | ESLint+Prettier, semicolons, single quotes, 100 chars, const>let, async/await, `interface`>`type`, `.test.ts` AAA |
| Python | Black 88, Google docstrings, type hints, pytest+fixtures |
| Bash | `set -euo pipefail`, quote vars, `local`, `trap cleanup EXIT` |
| CSS | Modules/BEM, `rem`/`px` borders, mobile-first, <=3 nesting |
| Config | 2-space indent |

## Writing

Tables>prose | Commands>descriptions | No preambles | Comments: WHY, <5% | Commits: conventional | PRs: Summary+Test plan | CHANGELOG: 1-line | ADR: 1-3 sentences

## Quality Gates

80% business / 100% critical coverage | Parameterized SQL | CSP | TLS 1.2+ | RBAC

## Fail-Loud (NON-NEGOTIABLE)

Empty unexpected data → `console.warn` + visible UI. Silent `return null` = BUG.

## Zero Debt (NON-NEGOTIABLE)

Done = ALL requirements + ALL verify + ALL touched files clean. REJECTED: "Out of scope" | Deferred | TODO/FIXME/stubs | Suppress lint | "Pre-existing"

## Limits

Max 250 lines/file | CLAUDE.md: 4000 tok | rules: 2000 tok | skills/agents: 1500 tok
