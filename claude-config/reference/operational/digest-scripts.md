<!-- v3.0.0 -->

# Digest Scripts

Digest scripts produce compact JSON (~10x less tokens), cached, enforced by `prefer-ci-summary.sh` hook (exit 2).

## Mapping (NON-NEGOTIABLE)

| Raw command | Digest |
|---|---|
| `gh run view --log-failed` | `service-digest.sh ci` |
| `gh pr view --comments` | `service-digest.sh pr` |
| `vercel logs` | `service-digest.sh deploy` |
| `npm install/ci` | `npm-digest.sh install` |
| `npm run build` | `build-digest.sh` |
| `npm audit` | `audit-digest.sh` |
| `npx vitest`/`npm test` | `test-digest.sh` |
| `git diff main...feat` | `diff-digest.sh main feat` |
| `npx prisma migrate` | `migration-digest.sh status` |
| merge/rebase conflicts | `merge-digest.sh` |
| stack traces | `cmd 2>&1 \| error-digest.sh` |
| `git status/log` | `git-digest.sh [--full]` |
| Copilot PR comments | `copilot-review-digest.sh` |
| audit/hardening/linters | `project-audit.sh --project-root $(pwd)` |
| `gh pr checks` | `ci-digest.sh checks <pr>` |
| CI polling | `ci-watch.sh <branch> --repo owner/repo` |
| SSH sync | `mesh-sync.sh [--peer NAME]` |
| Remote tasks | `mesh-exec.sh <peer> <prompt> [--model]` |
| Peer health | `mesh-health.sh [--peer NAME]` |
| Mesh preflight | `mesh-preflight.sh [--json] [--peer N]` |
| Auth sync | `mesh-auth-sync.sh [--check-only]` |
| DB migrations | `apply-migrations.sh` |

Options: `--no-cache` (fresh) | `--compact` (~30-40% fewer tokens)
