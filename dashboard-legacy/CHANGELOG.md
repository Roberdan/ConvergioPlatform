# Changelog

### Dashboard DS Full Alignment (MaranelloLuceDesign v4.0.2 + NaSra)

**CDN & Infrastructure**
- Upgraded CDN from **v3.3.0** to **v4.0.2** (CSS, integration CSS, IIFE, WC loader).
- Added new WCs to loader: `mn-theme-rotary`, `mn-mapbox`.

**Semantic Token Migration (NaSra-compliant)**
- Bridge CSS rewritten with `--mn-surface/text/border` semantic tokens cascading into shadow DOM.
- All JS palette hardcodes → `Maranello.palette()` live calls (mn-charts, mn-kpi, mn-viz).
- `--mn-text` everywhere, zero `--bianco-caldo` (v4.0.2 CI gate compliant).
- A11y scaling: `--mn-a11y-font-scale`/`--mn-a11y-space-scale` in cockpit labels.

**Full DS Component Adoption (14 renderer files)**
- **mission.js**: `mn-badge`, `mn-hover-lift`, `mn-anim-fadeInUp`, `mn-btn--sm`, `mn-progress-fill`, status dots
- **mission-details.js**: `mn-status-done/in-progress/pending` on flow steps, `mn-row-*` on wave bars
- **plan-kanban.js**: `mn-hover-lift`, `mn-badge`, `mn-progress-fill`, `mn-btn--sm`
- **task-pipeline.js**: `mn-row-done/in-progress/blocked/pending`, `mn-anim-fadeIn`
- **nightly-jobs.js**: `mn-night-agent`, `mn-hover-lift`, `mn-anim-fadeIn`, `mn-badge`, `mn-btn--sm`, `mn-status-*`
- **nightly-jobs-detail.js**: `mn-signal-panel`, `mn-stat-row`, `mn-badge`, `mn-btn--sm`, `mn-anim-fadeIn`
- **idea-jar.js**: `mn-hover-lift`, `mn-tag`, `mn-btn--sm`, `mn-card-dark`
- **optimize.js**: `mn-card-dark`, `mn-stat__value/label`, `mn-btn--sm`, `mn-badge`
- **chat-panel.js**: `mn-input`, `mn-select`, `mn-badge`, `mn-btn--accent`, `mn-anim-fadeIn`
- **kpi-modals.js**: `mn-card-dark`, `mn-hover-lift`
- **activity.js**: `mn-tag`, `mn-badge`, `mn-anim-fadeIn`, `mn-status-*`, `mn-hover-lift`
- **github-activity.js**: `mn-tag`, `mn-badge`, `mn-anim-fadeIn`

**Header Redesign**
- `<mn-system-status>` WC embedded directly in header HTML.
- `mn-gradient-text` for Convergio title (Ferrari accent gradient).
- `mn-status-dot--success` for live status indicator.
- Responsive: header collapses to single column on mobile ≤640px.

**KPI Cockpit Enhancements**
- Signal row: Mesh/Plans/Blocked health indicators with `mn-status-dot` variants.
- Sparkline trends: tokens, lines, cost via `Maranello.sparkline()` with `autoResize()`.
- Speedometer `size: 'fluid'` for responsive gauge rendering.

**Mesh Network Cleanup**
- Removed redundant circular canvas mesh visualization (`mn-viz.js enhanceMeshNetwork`).
- Removed dead `mesh.js` (canvas element never existed in HTML).
- Kept only the card strip + flow particles from `websocket.js`.

**Responsive Mobile (v4.0.2)**
- `autoResizeAll()` for canvas charts.
- `initSidebarToggleAuto()` for mobile hamburger sidebar.
- Cockpit stacks vertically on ≤640px, sparklines hidden on mobile.
- Migrated all theme tokens from primitive (`--nero-soft`, `--grigio-scuro`) to semantic (`--mn-surface`, `--mn-surface-raised`, `--mn-text`, `--mn-border`, etc.) — these cascade into shadow DOM for automatic WC theming.
- Replaced hardcoded color palettes in JS (`PALETTE`, `STATUS_COLORS`, LED/gauge/mesh colors) with live `Maranello.palette()` calls — colors now auto-adapt to runtime theme changes.
- Added responsive mobile support: `autoResizeAll()` for canvas charts, `initSidebarToggleAuto()` for mobile hamburger sidebar, cockpit panel responsive stacking at ≤640px.
- Added new WCs to loader: `mn-theme-rotary`, `mn-mapbox`.
- Bridge CSS rewritten with v4.0.2 semantic tokens + responsive `@media` overrides.
- Themes CSS maranello block uses `--mn-surface/text/border` tokens + `--signal-*` status aliases.
- Glass theme noted as local-only variant (removed from DS in v4.0.0).
- Applied v4.0.2 lesson: `--mn-text` instead of `--bianco-caldo` for theme-adaptive text.
- Added `--mn-a11y-font-scale`/`--mn-a11y-space-scale` integration in cockpit stat labels.

### Dashboard DS Alignment (MaranelloLuceDesign v3.3.0)

- Migrated dashboard integration from **v3.2.1** to **v3.3.0** CDN assets and aligned loader references.
- Added and wired `integration.css` from MaranelloLuceDesign to replace duplicated local styling.
- Reduced bridge CSS from the historical 593-line baseline down to a lean custom layer (now 78 lines).
- Hardened rendering paths with shared HTML escaping (`window.esc` + Maranello escape fallback) for XSS safety.
- Migrated widget and mesh naming to DS classes (`mn-widget*`, `mn-mesh-*`, `mn-convergio-toolbar`, `mn-convergio-pill`).
- Implemented DS-compatible data flow hooks (`bind`, `emit`, `bindChart`, `autoBind`) with safe fallbacks.
- Fixed runtime data consistency issues including network traffic mapping (`networkMessages`) and neural graph node updates (`neuralNodes`).

