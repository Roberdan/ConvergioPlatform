<!-- v1.0.0 — Merged: code-quality + zero-debt + file-size-limits + token-budget -->

# Code Standards

## Style

| Lang | Standard |
|---|---|
| TS/JS | ESLint+Prettier, semicolons, single quotes, 100 chars, const>let, async/await, `interface`>`type`, colocated `.test.ts` AAA |
| Python | Black 88, Google docstrings, type hints, pytest+fixtures |
| Bash | `set -euo pipefail`, quote vars, `local`, `trap cleanup EXIT` |
| CSS | Modules/BEM, `rem`/`px` borders, mobile-first, <=3 nesting |
| Config | 2-space indent |

## Writing

- Tables>prose. Commands>descriptions. No preambles (README excepted)
- Comments: WHY not WHAT, <5% lines
- Commits: conventional, 1 subject line. PRs: Summary+Test plan
- CHANGELOG: 1-line entries. ADR: 1-3 sentences

## REST API

- Plural nouns, kebab-case, max 3 levels, `/api/v1/`
- Status: 200/201/204 | 400/401/403/404/409/422/429/500/503
- Error: `{error:{code,message,details?,requestId,timestamp}}`
- Pagination: `?page=1&limit=20` (max 100). OAuth 2.0/JWT. OpenAPI

## Quality Gates

- 80% business / 100% critical coverage
- Parameterized SQL | CSP | TLS 1.2+ | RBAC
- A11y: 4.5:1 contrast, keyboard, screen readers, 200% resize

## Fail-Loud (NON-NEGOTIABLE)

- Empty unexpected data → `console.warn` + visible UI
- Silent `return null` = BUG

## Zero Debt (NON-NEGOTIABLE)

- Done = ALL requirements + ALL verify + ALL touched files clean
- Touch ANY line → own ALL issues
- REJECTED: "Out of scope" | Deferred | TODO/FIXME/stubs | Suppress lint | "Pre-existing" on touched file

## Limits

- Max 250 lines/file. Split if exceeds
- CLAUDE.md/AGENTS.md: max 4000 tokens | rules/*.md: max 2000 | skills/agents: max 1500
