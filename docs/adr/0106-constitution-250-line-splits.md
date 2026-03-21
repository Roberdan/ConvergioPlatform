# ADR 0106: 250-Line File Splits for Daemon Constitution

Status: Accepted | Date: 21 Mar 2026

## Context

The ConvergioPlatform constitution enforces a 250-line-per-file limit. Several daemon Rust files grew beyond this (e.g. `api_dashboard.rs` at 980 lines, `api_plan_db_query.rs`, `sse.rs`, `ws_pty.rs`, `state.rs`, `handlers.rs`). Monolithic files increase merge conflicts, slow `cargo check`, and make incremental compilation less effective.

## Decision

Split oversized files into submodule directories: each original file becomes `mod.rs` (re-exports + routing), with logic extracted into named peer files (`handlers.rs`, `formatters.rs`, `session.rs`, `stream.rs`, etc.) each under 250 lines. Public API surface is unchanged — callers import the same paths. Where a natural subdomain exists (e.g. `api_plan_db_lifecycle`, `api_workers`, `api_ipc`), a full `mod/` directory replaces the flat file. All splits verified with `cargo check` before commit.

## Consequences

- Positive: all files ≤250 lines; faster incremental compilation; clearer responsibility boundaries; constitution CI check passes
- Negative: 20+ refactor commits required; module declaration paths must be updated in `mod.rs` files; grep/symbol searches need to know the submodule structure
- Enforcement: `scripts/ci/check-constitution.sh` fails CI if any `.rs` file exceeds 250 lines
