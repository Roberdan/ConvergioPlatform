# ADR-0100: Evolution Engine — Autonomous Self-Optimisation

## Status
Accepted

## Context
The Convergio mesh requires continuous optimisation across latency, bundle size, agent cost,
and workload efficiency. Manual review cycles are too slow; ad-hoc scripts lack safety
guarantees. A structured self-improvement loop is needed.

## Decision
Introduce a hypothesis-driven Evolution Engine that: (1) collects telemetry across 7 metric
families, (2) evaluates anomalies and opportunities via domain-specific evaluators, (3)
generates scored proposals with blast-radius classification, (4) validates proposals via
Shadow/Canary/BlueGreen experiments, and (5) applies changes **only via PR + experiment**
— never through direct production writes. A Guardrails layer (KillSwitch, RateLimiter,
SafetyValidator, AuditTrail) enforces safety invariants at every stage.

## Consequences
- Optimisations are traceable: every change links to a proposal, experiment, and outcome
- No production surprise: all changes reviewable before merge
- Blast-radius gates prevent unreviewed cross-repo changes
- Requires SQLite on M1; engine inactive if db unavailable
- Human review still mandatory for MultiRepo and Ecosystem blast radius
