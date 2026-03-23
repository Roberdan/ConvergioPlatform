# ADR 0108 — Workspace Layer

**Status**: Accepted
**Date**: 23 Marzo 2026
**Decision**: Introduce daemon-managed workspace layer. Git becomes invisible to agents.

## Context
Session 2026-03-22/23 revealed that worktree/branch/PR/merge friction caused repeated issues.
Git is a persistence format, not an agentic collaboration protocol. Agents should not manage
git operations directly.

## Decision
- Agents edit files via Read/Edit/Write tools normally — hooks register ops in daemon event log
- Daemon manages worktrees internally — agents see opaque workspace_id
- Release Agent (Rust module) automates: quality gate → commit → push → PR → merge
- GitConnector trait abstracts git provider (GitHub/GitLab/Gitea)
- Bash scripts (worktree-create.sh, wave-worktree.sh, pr-ops.sh) deprecated, migrated to Rust

## Consequences
- Positive: zero git friction for agents, audit trail, provider-agnostic
- Negative: daemon must be running for workspace ops
- Migration: old scripts stay as thin wrappers emitting deprecation warnings
