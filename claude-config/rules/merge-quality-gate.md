# Merge Quality Gate (NON-NEGOTIABLE)

All gates required. No `--admin` bypass. `pre-merge-gate.sh` enforces 1-5.

| # | Gate | Command | Block if |
|---|---|---|---|
| 1 | Clean tree | `git status --short` | Modified/untracked |
| 2 | No contamination | `git diff --name-only` vs task files | Outside scope |
| 3 | Type-check | `npx tsc --noEmit` / `cargo check` | Exit != 0 |
| 4 | Tests | `pytest` / `vitest` / `cargo test` | Exit != 0 |
| 5 | Lint | `ruff check` / `eslint` / `clippy` | Errors |
| 6 | Version | VERSION.md = pyproject.toml/package.json/Cargo.toml | Mismatch |
| 7 | CHANGELOG | Latest entry = current version | Stale |
| 8 | Stashes | `git stash list` | Orphan stashes |

Contamination: `git checkout -- <file>`. Post-merge: checkout main → pull → delete branch → drop stashes → verify worktree list.
