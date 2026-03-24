---
id: ADR-001
title: Adopt Maranello Luce Design System for CommandCenter
status: Accepted
date: 2026-03-24
---

# ADR-001: Adopt Maranello Luce Design System for CommandCenter

## Status

Accepted

## Context

CommandCenter had no centralized design system: colors, typography, and spacing were inlined across 65 Swift source files, making theming and brand consistency impossible to maintain. The existing Maranello Luce Design System (already adopted for the web dashboard) defines a canonical token vocabulary covering palette, typography scale, and semantic roles.

## Decision

Translate Maranello CSS design tokens to Swift and build a layered component library:

1. `ConvergioTokens` — raw color/spacing/radius constants mirroring Maranello palette
2. `ConvergioTokens+Roles` — semantic role aliases (surface, primary, accent, destructive)
3. `Typography` — `Font` extensions and `Spacing`/`CornerRadius` constants
4. `ThemeManager` — observable object injected via SwiftUI environment, supporting Editorial, Nero, and Avorio themes
5. Component library — `ConvergioCard`, `StatusBadge`, `SectionHeader`, `AccentButton`

All views are updated to consume tokens exclusively; no inline color literals remain.

## Consequences

- Single source of truth for brand colors eliminates per-file drift.
- Three themes (Editorial, Nero, Avorio) with runtime switching via `ThemeManager`.
- WCAG 2.1 AA contrast compliance enforced at the token level.
- New views must use `ConvergioTokens`/`Typography` — raw literals are rejected in review.
- Token translation must be kept in sync when the Maranello Luce Design System updates.
