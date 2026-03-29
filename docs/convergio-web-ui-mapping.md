# convergio-web — UI Mapping Specification

> Reference mapping for nasra-app-builder: how convergio-web daemon APIs should map
> to convergio-design components. Generated from CKB v6.2.0 analysis.

## Current State

convergio-web (Next.js 15 + Tauri 2) imports **only CSS tokens** from convergio-design.
Zero Web Components or TS APIs are used. All UI is custom Tailwind + Framer Motion.

## Target State

Replace custom UI with convergio-design components for consistency, accessibility,
and theme support across all 6 themes.

## App Shell

| Current | Target | Component |
|---------|--------|-----------|
| Custom sidebar (`app-shell.tsx`) | 4-slot grid layout | `createLayout()` from `@convergio/design-elements` |
| Custom header in sidebar | Rich header bar | `mn-header-shell` with brand, search, theme, profile sections |
| Custom CommandPalette | DS command palette | `mn-command-palette` |
| No theme toggle | Theme switching | `mn-theme-toggle` embedded in header-shell |
| Custom skeletons | Standard loading | `StateScaffold` via `DashboardRenderer` |

## Page Mappings

### `/` Dashboard (page.tsx)

| Endpoint | Current UI | Target Component | CKB Rule |
|----------|-----------|-----------------|----------|
| `GET /api/overview` | Custom KPI grid (6 cards) | `dashboardStrip` (board zone) + `mn-gauge` | `numeric-kpis` + `monitoring-strip` |
| `GET /api/mission` | Custom plan cards | `mn-data-table` (grouped by status) + `mn-gantt` (timeline) | `list-to-table` + `timeline-planning` |
| `GET /api/agents` (sidebar) | Custom running agent list | `mn-data-table` (compact) | `list-to-table` |
| `GET /api/mesh` (sidebar) | Custom peer status dots | `mn-system-status` (compact) | `health-check` |
| `GET /api/metrics/summary` | Inline numbers | `tokenMeter` + `kpiScorecard` | `numeric-kpis` |
| Polling (5s) | Custom `useEffect` + `useState` | `use-poll.ts` generic hook | — |

### `/agents` (agents/page.tsx)

| Endpoint | Current UI | Target Component | CKB Rule |
|----------|-----------|-----------------|----------|
| `GET /api/agents` | Custom agent cards + search input | `FacetWorkbench` (search + status filter) + `mn-data-table` | `filterable-table` |
| `GET /api/agents/catalog` | Custom collapsible category grid | `mn-data-table` (grouped by category) | `list-to-table` |

### `/mesh` (mesh/page.tsx)

| Endpoint | Current UI | Target Component | CKB Rule |
|----------|-----------|-----------------|----------|
| `GET /api/mesh` | Custom peer cards | `mn-data-table` + status badges | `list-to-table` |
| `GET /api/mesh/topology` | Custom SVG placeholder | `neuralNodes` (force-directed graph) | `topology-graph` |
| `GET /api/mesh/traffic` | Not shown | `mn-chart` (sparkline per peer) | `time-series` |

### `/brain` (brain/page.tsx)

| Endpoint | Current UI | Target Component | CKB Rule |
|----------|-----------|-----------------|----------|
| `WS /ws/brain` | Custom canvas (`brain-graph.tsx`) | `neuralNodes` + `socialGraph` | `topology-graph` |

### `/chat` (chat/page.tsx)

| Endpoint | Current UI | Target Component | CKB Rule |
|----------|-----------|-----------------|----------|
| `POST /api/chat/session` | Custom session create | API client (keep) | — |
| `POST /api/chat/message` + SSE | Custom chat bubbles | `mn-chat` (aiChat) | `streaming-chat` |
| Session sidebar | Custom mock list | `mn-data-table` (compact, selectable) | `list-to-table` |

### `/evolution` (evolution/page.tsx)

| Endpoint | Current UI | Target Component | CKB Rule |
|----------|-----------|-----------------|----------|
| `GET /api/evolution/roi` | Custom KPI cards | `dashboardStrip` (board zone) | `numeric-kpis` |
| `GET /api/evolution/proposals` | Custom proposal cards | `mn-data-table` + `approvalChain` | `list-to-table` |
| `GET /api/evolution/experiments` | Custom experiment list | `mn-data-table` | `list-to-table` |
| Pipeline stepper | Custom lifecycle component | `mn-customer-journey` | — |

### `/workspaces` (workspaces/page.tsx)

| Endpoint | Current UI | Target Component | CKB Rule |
|----------|-----------|-----------------|----------|
| `GET /api/workspaces` | Custom workspace cards | `mn-kanban-board` (by status) or `mn-data-table` | `task-statuses` |

## Components Used After Rebuild

| Component | Pages Using It |
|-----------|---------------|
| `mn-header-shell` | All (layout) |
| `createLayout` | All (layout) |
| `mn-theme-toggle` | All (in header) |
| `mn-command-palette` | All (global) |
| `mn-gauge` | Dashboard |
| `dashboardStrip` | Dashboard, Evolution |
| `mn-data-table` | Dashboard, Agents, Mesh, Evolution, Workspaces, Chat |
| `mn-gantt` | Dashboard |
| `FacetWorkbench` | Agents |
| `neuralNodes` | Mesh, Brain |
| `mn-chat` | Chat |
| `mn-customer-journey` | Evolution |
| `mn-kanban-board` | Workspaces |
| `mn-system-status` | Dashboard (sidebar) |
| `tokenMeter` | Dashboard |
| `approvalChain` | Evolution |
| `mn-chart` (sparkline) | Mesh, Dashboard |

**Total: 17+ unique DS components** (vs current 0)

## What to Preserve

- `src/lib/daemon-api.ts` — API client is clean and well-typed, keep as-is
- `src/lib/daemon-types.ts` — TypeScript interfaces are complete, keep as-is
- `src-tauri/` — Tauri config is working, keep as-is
- `e2e/` — Test structure, update assertions for new components
- `tailwind.config.ts` — Keep for utility classes alongside DS components
