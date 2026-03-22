# ADR-0202: Deliverable Management Architecture

**Status**: Accepted
**Date**: 22 Marzo 2026
**Plan**: #689 — Plan B (Deliverable Management, Project CLI, Audit, Skills)
**Deciders**: Roberto D'Angelo

---

## Context

ConvergioPlatform lacked a formal way to track deliverables (documents, reports, artifacts) produced by agents during plan execution. Outputs were scattered across worktrees with no versioning, approval workflow, or filesystem convention. Agents had no way to declare "this artifact is ready for review" or get consent before writing to shared paths.

## Decision

**Deliverables are first-class entities with DB tracking, filesystem output, and consent gates.**

| Aspect | Design |
|---|---|
| Storage | DB table `deliverables` (id, plan_id, task_id, type, status, path, version, created_at) |
| Filesystem | Output to `data/deliverables/{project}/{plan_id}/` with versioned filenames |
| Lifecycle | `draft` → `submitted` → `approved` (requires explicit consent) |
| CLI | `cvg deliverable create`, `cvg deliverable approve`, `cvg deliverable version` |
| Consent | Agent must request approval before writing to shared/external paths; no silent overwrites |

## Consequences

**Positive:**
- Audit trail for all agent-produced artifacts
- Versioned outputs prevent accidental overwrites
- Consent gate aligns with Constitution Article II (Safety) and Article IV (Transparency)
- Project-scoped paths keep deliverables organized

**Negative:**
- Agents must use `cvg deliverable create` instead of direct file writes for tracked outputs
- Additional DB table and migration required

---

*Per Constitution Article IV (Transparency) and Article VI (Verification): deliverables require explicit consent and approval before marking done.*
