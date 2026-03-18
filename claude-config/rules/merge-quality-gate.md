<!-- v2.0.0 -->
# Merge Quality Gate (NON-NEGOTIABLE)

Every merge MUST pass ALL gates. No `--admin` bypass. Hook `pre-merge-gate.sh` enforces 1-5.

| # | Gate | Command | BLOCK if |
|---|---|---|---|
| 1 | Clean tree | `git status --short` | Modified/untracked |
| 2 | No contamination | `git diff --name-only` vs task files | Outside scope |
| 3 | Type-check | `npx tsc --noEmit -p tsconfig.app.json` | Exit != 0 |
| 4 | Tests | `pytest -m "not integration"` / `vitest` | Exit != 0 |
| 5 | Lint | `ruff check` / `eslint` | Errors |
| 6 | Version | VERSION.md = pyproject.toml/package.json | Mismatch |
| 7 | CHANGELOG | Latest entry = current version | Stale |
| 8 | Stashes | `git stash list` | Orphan stashes |

Contamination: `git checkout -- <file>` or `git stash push -m "other" -- <files>`. NEVER commit outside-scope files.

Post-merge: `git checkout main && git pull` → delete branch → drop stashes → verify `git worktree list` (only main). _Why: Plan v21._
