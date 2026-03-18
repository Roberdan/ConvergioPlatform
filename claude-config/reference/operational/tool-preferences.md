<!-- v4.0.0 -->

# Tool Preferences

## Tool Mapping

| Task | Use | NOT |
|------|-----|-----|
| Find file | Glob | `find`, `ls` |
| Search content | Grep | `grep`, `rg` |
| Read/Edit/Create | Read/Edit/Write | `cat`/`sed`/`echo >` |
| Definition/Usages | LSP | Grep |
| Symbol search | `codegraph_search` (if `.codegraph/`) | Grep |
| Explore codebase | `Task(subagent_type='Explore')` | Multiple grep/glob |
| Audit | `project-audit.sh --project-root $(pwd)` | Manual scripts |

ALWAYS parallelize independent tool calls in single message. Subagent routing: see `reference/operational/agent-routing.md`.

## Shell Safety (zsh)

Single-quote URLs with `?`/`&` | NEVER `!=` in SQL (use `<>`/`NOT IN`, hook blocks) | Fork PRs: `gh api 'repos/{o}/{r}/pulls'`

## CI/Build (MANDATORY)

| AVOID | USE |
|-------|-----|
| `npm run lint/typecheck/build/test` | `ci-summary.sh --lint/--types/--build/--unit` |
| `gh run view --log` | `ci-digest.sh <id>` |
| `gh pr checks` | `ci-digest.sh checks <pr>` |
| `gh pr view/merge` | `pr-ops.sh status/merge <pr>` |
| `git log` verbose | `git log --oneline -N` |

Hook `prefer-ci-summary.sh` enforces. GitHub auth: 404 → `gh auth switch` | 401 → `gh auth refresh`
