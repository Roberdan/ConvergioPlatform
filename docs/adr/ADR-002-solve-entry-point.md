# ADR-002: /solve as Mandatory Entry Point

**Status**: Accepted | **Date**: 2026-03-22

**Context**: /prompt was too thin — no triage, no compliance check, no problem understanding.
**Decision**: /solve replaces /prompt with 10-phase consultant workflow. /prompt deprecated.
**Consequences**: All complex work flows through /solve → /planner → /execute.
