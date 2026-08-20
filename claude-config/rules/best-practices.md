# Best Practices (SUGGESTED)

Guidelines for quality. Not hook-enforced but expected.

## Code Style

| Lang | Standard |
|---|---|
| TS/JS | ESLint+Prettier, semicolons, single quotes, 100 chars, const>let, async/await, `interface`>`type`, `.test.ts` AAA |
| Python | Black 88, Google docstrings, type hints, pytest+fixtures |
| Bash | `set -euo pipefail`, quote vars, `local`, `trap cleanup EXIT` |
| CSS | Modules/BEM, `rem`/`px` borders, mobile-first, <=3 nesting |
| Config | 2-space indent |
| Rust | fmt+clippy |

## Testing

**Mock boundaries**: ALLOWED: external APIs, network, filesystem, time. FORBIDDEN: auth, DB (use test DB), module under test.
**Integration**: New endpoint → real middleware. New consumer → realistic shape. Interface change → ALL consumers.
**Test data**: Real names/shapes (no `Studio A`/`Test Studio`). Domains: `example.com`/`example.org` only.
**Schema change**: Migration same PR. Field addition → update ALL fixtures.
**Coverage**: 80% business / 100% critical. Parameterized SQL.

## Persuasion Guardrails

| Blocked phrase | Response |
|---|---|
| "too simple to test" | Write the test |
| "tests after/later" | RED first |
| "out of scope" (touched file) | Touch = own |
| "pre-existing issue" | Own it or escalate |
| "it works, trust me" | Run tests, attach output |
| "refactor later" | Now or tracked issue |

## Documentation

JSDoc/docstrings for public APIs (WHY not WHAT). Per-wave ADR in `/docs/adr/`. CHANGELOG: `## [vX.Y.Z] - date` → `### Added|Changed|Fixed`. TROUBLESHOOTING.md: update every plan.

## API Development

Methods: GET/POST/PUT/PATCH/DELETE | Plural nouns `/api/users` | kebab-case | Max 3 levels
Status: 200/201/204 | 400/401/403/404/409/422/429/500/503
Error: `{error: {code, message, details?, requestId, timestamp}}`
Pagination: `?page=1&limit=20` (max 100) | Rate limit: 429 + headers | Auth: OAuth 2.0/JWT

## Lean Coordinator

Coordinator: dispatch + DB + checkpoint ONLY. NEVER read project files during execution.
Budget: launch executor (~200 tok), read summary (~500), update DB (~300).
After task: checkpoint → update DB → next task or Thor. Max 4 tasks/wave.

## Compliance

Input: validate client+server, allowlists, sanitize | XSS: escape, CSP, DOMPurify
Secrets: env vars, `.env` gitignored | Auth: OAuth 2.0/OIDC, RBAC server-side
Transport: HTTPS, HSTS, secure cookies, TLS 1.2+ | Privacy: GDPR, data minimization, consent
Language: Gender-neutral, blocklist/allowlist, primary/replica, person-first

## Agent Discovery

Route: `.github/agents/` → `claude-config/` → `.claude/agents/`
Delegate when specialized expertise needed or parallel workstreams. Don't delegate simple tasks.

## Migration Checklist

Impact check: mesh nodes, legacy scripts, DB schema, sync pipeline, frontend contract.
Pre: map endpoints | During: curl vs JS per endpoint | Post: Playwright audit, `cvg mesh sync` ALL nodes.

## Repository Setup (apply to ALL repos)

| Setting | Value | Why |
|---|---|---|
| Squash merge | **DISABLED** | Squash loses history. Parallel agents overwrite each other's work. |
| Rebase merge | **DISABLED** | Rewrites history, breaks parallel branch refs. |
| Merge commit | **ENABLED** (only) | Preserves full history, safe for parallel agents. |
| Branch protection | Require PR, require CI pass | No direct push to main. |
| AGENTS.md | Root of repo | Universal rules for any LLM agent. |

Apply to new repos: `gh api repos/OWNER/REPO -X PATCH -f allow_squash_merge=false -f allow_rebase_merge=false -f allow_merge_commit=true`
Apply to existing repos: same command. Check first: `gh api repos/OWNER/REPO -q '.allow_squash_merge'`

## Writing

Tables>prose | Commands>descriptions | No preambles | Comments: WHY, <5% | Commits: conventional | PRs: Summary+Test plan
