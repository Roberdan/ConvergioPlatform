# ADR-003: Universal Skill Protocol v1.0

**Status**: Accepted | **Date**: 2026-03-22

**Context**: Skills were Claude Code-specific. Need portability across providers.
**Decision**: Two-file format (skill.yaml + SKILL.md) with transpilers per provider.
**Consequences**: Skills are provider-agnostic. Transpilers generate provider-specific format.
