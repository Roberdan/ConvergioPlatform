# ADR-0113: Hook Consolidation for Context Window Stability

## Status: Accepted
## Date: 25 Marzo 2026

## Context
Each PreToolUse hook invocation generates a progress event in Claude's context window. With 13 hooks and 80+ tool calls per session, this produced 1000+ events causing context window exhaustion and `cache_control.ttl` ordering bugs.

## Decision
Consolidate 13 PreToolUse hooks into a single dispatcher script `scripts/platform/pre-tool-guard.sh`, reducing to 3 hooks (~77% reduction in context events). Commit e0d4692.

## Consequences
- Context event volume reduced by ~77%, extending effective session length
- Single dispatcher is easier to maintain and debug than 13 separate scripts
- All hook logic preserved; dispatch is by tool name internally
- TTL ordering errors eliminated by reducing concurrent progress events
