---
applyTo: "**/*.rs"
---

# Rust Coding Standards

- Format: `cargo fmt` (rustfmt defaults)
- Lint: `cargo clippy -- -D warnings` (zero warnings)
- Max 250 lines per file
- Error handling: `thiserror` for library errors, `anyhow` for binary
- Async: `tokio` runtime, prefer `async fn` over manual futures
- SQLite: `rusqlite` with `PRAGMA busy_timeout=5000` on all connections
- Tests: `#[cfg(test)]` modules with `#[path]` for separate test files
- No `unwrap()` in production code — use `?` or `.expect("reason")`
- Comments: WHY not WHAT, under 5% of lines
- Naming: `snake_case` for functions/modules, `PascalCase` for types
