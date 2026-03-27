<!-- Copyright (c) 2026 Roberto D'Angelo. MPL-2.0. -->
# ADR-0117: Convergio UI Platform Strategy

**Date:** 27 Marzo 2026
**Status:** Accepted
**Supersedes:** ADR-0101 (dashboard rebuild), ADR-0110 (CommandCenter SwiftUI)

## Context

Convergio has three inconsistent UI surfaces that fail to expose the platform's capabilities (130+ API endpoints, 89+ agents, evolution engine, mesh P2P, voice pipeline, CRDT sync):

| Surface | Tech | Problem |
|---|---|---|
| TUI | Ratatui (Rust) | 11 views, most complete, but ASCII-only, no rich media |
| Mac menu bar | SwiftUI + 70% WebView | Hybrid, chat not connected, maintenance burden |
| Dashboard | Vanilla JS + Maranello IIFE | Minimal (channels panel only), rebuild stalled |

None of these surfaces adequately renders: AI chat streaming, voice interaction, mesh topology, agent brain graph, evolution proposals, or workspace orchestration.

A parallel initiative proposed transforming Maranello into a generic app framework (SvelteKit + shadcn-svelte + `create-maranello-app`). This was rejected as premature — only one consumer (VirtualBPM) existed, and the AI-specific requirements of Convergio (chat, voice, agent UI, streaming) are better served by React/Next.js ecosystem tooling.

## Decision

### Piano A: Convergio UI first

Build the Convergio UI as the primary investment. Maranello is pacchettized as foundation, not as a generic framework.

**Confirmed stack:**

| Surface | Technology | Rationale |
|---|---|---|
| Web (primary) | Next.js + Vercel AI SDK 6 + shadcn/ui | AI Elements (chat), Voice Elements (TTS/transcription), agent tool approval, streaming — purpose-built for Convergio's use case |
| Desktop Mac | Tauri 2.0 wrapping the same Next.js app | Replaces CommandCenter SwiftUI. <10MB bundle, native Rust affinity, 90%+ code sharing with web |
| Mobile | PWA from Next.js (deferred) | Installable, offline-capable. Native mobile deferred until PMF proven |
| Terminal | Ratatui (keep as-is) | Already functional, 11 views, ops-focused |
| Design foundation | @maranello/tokens + @maranello/elements | Ferrari Luce aesthetic preserved. CSS variables + 36 web components |

**Rejected alternatives:**

| Option | Why rejected |
|---|---|
| SvelteKit + shadcn-svelte | Vercel AI SDK 6 is React/Next.js first; Svelte support partial |
| Flutter | Dart ecosystem excludes JS AI tooling (Vercel AI SDK, LangChain.js) |
| Electron | 100MB+ bundles, high RAM; Tauri does the same at 1/10 weight |
| SwiftUI + KMP | 3-4 dev team minimum; current SwiftUI app is 70% webview already |
| Framework-first (create-maranello-app) | Premature — one consumer, framework should emerge from real experience |

### Maranello pacchettizzazione (foundation)

Extract from existing MaranelloLuceDesign monolith into two npm packages:

| Package | Contents |
|---|---|
| @maranello/tokens | CSS variables, 5 themes, theme switcher, bridge CSS for shadcn/ui |
| @maranello/elements | 36 Web Components (mn-gauge, mn-chart, mn-data-table, mn-gantt, etc.) + headless TS, per-element entry points, tree-shakeable |

**What gets killed:**

| Component | Replacement |
|---|---|
| AppShellController | Next.js layouts |
| ViewRegistry | Next.js file-based routing |
| NavigationModel | Next.js router |
| StateScaffold | Next.js loading.tsx + error.tsx |
| CommandCenter (SwiftUI) | Tauri 2.0 wrapping Next.js |
| Vanilla JS dashboard | Next.js app |
| Maranello IIFE bundle | npm packages with tree-shaking |

**What stays (unique Maranello value):**

- 5 Ferrari Luce themes via CSS variables
- 36 domain-specific web components (gauges, charts, kanban, gantt, heatmap, network viz, etc.)
- WCAG 2.2 AA compliance
- Ratatui TUI (terminal surface)
- Daemon Rust API (:8420) — unchanged

### Piano B: Maranello Framework (deferred)

After Convergio UI is validated, extract the pattern into a generic framework (`create-maranello-app`, docs, cookbook). The framework technology (Next.js vs SvelteKit) will be decided based on real Convergio experience, not upfront.

### Architecture

```
                    Rust Daemon (:8420)
                   REST / WebSocket / SSE
                           |
           +-------+-------+-------+-------+
           |       |       |       |       |
        Next.js  Tauri   Mobile  Ratatui  OpenClaw
         (Web)  (Desktop) (PWA)   (TUI)  (Chat bots)
           |       |
           +---+---+
               |
    @maranello/tokens + @maranello/elements
         (design foundation)
```

## Consequences

- ADR-0101 (dashboard rebuild with Maranello Presentation Runtime) is superseded — the Presentation Runtime is replaced by Next.js.
- ADR-0110 (CommandCenter SwiftUI) is superseded — the native Mac app is replaced by Tauri 2.0.
- The TUI (Ratatui) remains the ops-focused terminal surface and is unaffected.
- All new Convergio UI work targets the Next.js app. No new vanilla JS dashboards or SwiftUI investment.
- Maranello evolves from a design system monolith into two focused npm packages consumed by Next.js.
- The daemon API surface (:8420) remains the single source of truth for all UI surfaces.
- Tauri 2.0 mobile support (maturing mid-2026) provides a future path to native mobile from the same codebase. If it doesn't mature, a thin React Native shell calling the REST API is the escape hatch.
