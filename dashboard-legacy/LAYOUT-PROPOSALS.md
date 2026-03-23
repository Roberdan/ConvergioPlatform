# Convergio Control Room — Layout Redesign Proposals

**Author**: Sara — Senior UX/UI Designer
**Date**: 09 Luglio 2025
**Status**: RFC (Request for Comments)
**Scope**: Dashboard grid restructure, widget grouping, KPI bar redesign, responsive strategy

---

## Current State Analysis

### Measured Problems

| # | Issue | Impact | Severity |
|---|-------|--------|----------|
| 1 | 16 widgets in 2 columns → ~3200px vertical scroll | Users never see bottom widgets | Critical |
| 2 | Left column 33% → Task Pipeline table cramped at ~400px | Data truncation, horizontal scroll | High |
| 3 | KPI bar: 8 cards + 3 gauges in one flex-wrap row | Cognitive overload, no scan path | High |
| 4 | Brain widget 720px fixed height → 50%+ of right column | Dominates layout, pushes critical data below fold | High |
| 5 | Mesh + Latency = 2 separate widgets (~440px combined) | Related data split, wastes vertical space | Medium |
| 6 | Charts (Task Dist, Cost, Token Burn) buried at scroll bottom | Analytics invisible, zero engagement | High |
| 7 | No grouping — missions, plans, mesh all same visual weight | No information hierarchy, no scan pattern | Critical |
| 8 | Idea Jar pinned at left column bottom | Unreachable without full scroll | Low |

### Viewport Budget (1920×1080 reference)

```
Header:        ~56px
KPI Bar:      ~140px
Available:    ~884px (below fold = wasted for critical data)
```

At 884px available, only 3-4 widgets fit above the fold per column.
Current layout puts 8 widgets per column — **75% of content is below fold**.

---

## Design Principles for Redesign

1. **Above-the-fold rule**: Mission status, active tasks, mesh health, and cost — always visible without scrolling
2. **Progressive disclosure**: Overview cards → click/expand for detail tables and charts
3. **Semantic grouping**: Related data in composite widgets, not scattered
4. **Asymmetric grid**: Variable column widths based on content density, not fixed 33/67
5. **Brain as ambient**: Neural viz should be a background/sidebar feature, not the centerpiece
6. **Scan pattern**: Left-to-right, top-to-bottom Z-pattern for critical metrics

---

## Proposal A: "Mission Control Grid"

> Inspired by NASA JPL Mission Control — dense grid with focal zones

### Grid Structure

```
4-column grid: 1fr 1fr 1fr 1fr
Rows: auto-sized with explicit grid-area placement
Gap: 12px
Padding: 16px 20px
```

```css
.dash-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  grid-template-rows: auto auto auto auto;
  grid-template-areas:
    "status  status  status  status"
    "ops     ops     mesh    brain"
    "tasks   tasks   charts  brain"
    "timeline timeline timeline timeline";
  gap: 12px;
  padding: 16px 20px;
}
```

### ASCII Layout (1920px viewport)

```
┌─────────────────────────────────────────────────────────────────────┐
│  HEADER: [Overview][Admin][Planner][Brain][IdeaJar]   Convergio  ⟳☰│
├─────────────────────────────────────────────────────────────────────┤
│                    COMMAND STRIP (redesigned KPI)                    │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ │ ◎Agents │ ┌────┐ ┌────────┐│
│  │Active: 3│ │Mesh: 3/3│ │Tokens:2M│ │  ◎ 5    │ │$4.2│ │Blocked:0││
│  │plans    │ │nodes on │ │today:80K│ │ running │ │cost│ │▓▓░░ 82%││
│  └─────────┘ └─────────┘ └─────────┘ │         │ └────┘ └────────┘│
├──────────────────────────┬────────────┬─────────┴──────────────────┤
│                          │            │                            │
│   OPS ZONE               │  MESH HUB  │     BRAIN                  │
│ ┌──────────────────────┐ │ ┌────────┐ │  ┌────────────────────┐   │
│ │ Active Missions (3)  │ │ │worker-1 │ │  │                    │   │
│ │ ▸ Plan-291 building..│ │ │CPU ▓▓░ │ │  │   Neural Network   │   │
│ │ ▸ Plan-289 testing.. │ │ │RAM ▓░░ │ │  │   Visualization    │   │
│ │ ▸ Plan-287 deploying │ │ │↕ 12ms  │ │  │                    │   │
│ │                      │ │ ├────────┤ │  │   (420px height)   │   │
│ │ Last Missions (24h)  │ │ │worker-2   │ │  │                    │   │
│ │ · Plan-285 ✓ done    │ │ │CPU ▓░░ │ │  │   Compact mode:    │   │
│ │ · Plan-284 ✓ done    │ │ │RAM ▓▓░ │ │  │   nodes as dots,   │   │
│ └──────────────────────┘ │ │↕ 8ms   │ │  │   expand on hover  │   │
│                          │ ├────────┤ │  │                    │   │
│                          │ │linux-worker │ │  └────────────────────┘   │
│                          │ │CPU ▓▓▓ │ │                            │
│                          │ │RAM ▓▓░ │ │                            │
│                          │ │↕ 15ms  │ │                            │
│                          │ └────────┘ │                            │
├──────────────────────────┼────────────┴────────────────────────────┤
│                          │                                         │
│   TASK ZONE              │     ANALYTICS TRIPTYCH                  │
│ ┌──────────────────────┐ │  ┌──────────┬──────────┬──────────┐    │
│ │ Task Pipeline        │ │  │TaskDist  │Cost/Model│TokenBurn │    │
│ │ ┌────┬──────┬──────┐ │ │  │  ◔       │ ▓▓▓     │ ╱╲       │    │
│ │ │ ID │Status│Plan  │ │ │  │  pie     │ stacked │ ╱  ╲╱╲   │    │
│ │ ├────┼──────┼──────┤ │ │  │  chart   │ bar     │ area     │    │
│ │ │ T1 │▸exec │ 291  │ │ │  │          │         │ chart    │    │
│ │ │ T2 │ pend │ 291  │ │ │  │          │         │          │    │
│ │ │ T3 │▸exec │ 289  │ │ │  └──────────┴──────────┴──────────┘   │
│ │ └────┴──────┴──────┘ │ │                                        │
│ └──────────────────────┘ │                                         │
├──────────────────────────┴─────────────────────────────────────────┤
│                      TIMELINE STRIP                                │
│ ┌──────────────────────────────────────────────────────────────┐   │
│ │ ◇ Plan-291   ◇ Plan-289   ◇ Plan-287   [Kanban ▸]  [Events]│   │
│ │ ▓▓▓▓▓▓░░░░   ▓▓▓▓▓▓▓▓░░   ▓▓▓▓░░░░░   GitHub ▸    Nightly│   │
│ └──────────────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────────┘
```

### Widget Grouping & Merges

| Current Widget(s) | Proposal A Treatment | Rationale |
|---|---|---|
| Active Missions + Last Missions | **Merge → "Ops Zone"** composite widget with tab: Active / Recent / History | Same data domain, eliminates 2 widget chrome |
| Mesh Network + Mesh Latency | **Merge → "Mesh Hub"** single widget: node cards include latency inline (↕ Xms badge) | Latency IS mesh data; separate widget wastes 160px |
| Task Distribution + Cost by Model + Token Burn | **Merge → "Analytics Triptych"** side-by-side mini-charts in one widget | Related analytics, 3 tiny charts > 3 full widgets |
| Plan Timeline + Plan Pipeline | **Merge → "Timeline Strip"** with toggle: Gantt / Kanban view | Same data (plans), different views |
| GitHub Activity + Nightly Jobs + Event Feed | **Merge → "Activity Feed"** tabbed: GitHub / Nightly / Events | All are event streams, low-priority |
| Augmented Brain | **Shrink to 420px**, right sidebar, compact node display | Still prominent but doesn't dominate |
| Idea Jar | **Move to nav tab** (already exists as pill) — remove from grid | Accessed via tab, not inline widget |

### Visual Hierarchy

```
TIER 1 (always visible, above fold):
  → Command Strip KPIs
  → Ops Zone (active missions)
  → Mesh Hub (system health)

TIER 2 (visible with minimal scroll):
  → Task Pipeline (the work)
  → Analytics Triptych (the trends)
  → Brain (ambient awareness)

TIER 3 (progressive disclosure / tabs):
  → Timeline Strip (plan progress)
  → Activity Feed (events, GitHub, nightly)
  → History (accessible from Ops Zone tab)
  → Idea Jar (nav tab)
```

### KPI Bar Redesign: "Command Strip"

Replace the flex-wrap chaos with a **structured 3-section strip**:

```
┌──────────────────────────────────────────────────────────────────┐
│ MISSION          │ AGENTS GAUGE │ OPERATIONS                     │
│ ┌─────┐ ┌─────┐ │              │ ┌─────┐ ┌─────┐ ┌───────────┐ │
│ │  3  │ │ 3/3 │ │    ◎  5     │ │ 2.1M│ │$4.20│ │ ▓▓▓▓░░ 82%│ │
│ │plans│ │nodes│ │   running   │ │token│ │cost │ │ progress  │ │
│ │activ│ │ on  │ │              │ │today│ │today│ │ 127 lines │ │
│ └─────┘ └─────┘ │              │ └─────┘ └─────┘ └───────────┘ │
│                  │              │ blocked: 0  lines/wk: 1.2K    │
├──────────────────┴──────────────┴────────────────────────────────┤
```

**Structure**:
- **Left zone — Mission**: Active Plans + Mesh Nodes (the "are we online?" glance)
- **Center — Hero Gauge**: Single large Agents gauge (the heartbeat) — replaces 3 gauges
- **Right zone — Operations**: Tokens, Cost, Lines, Blocked (the "how much?" details)
- **One gauge instead of three**: Plans count is a number, mesh count is a number. Only agents need a live gauge.

```css
.command-strip {
  display: grid;
  grid-template-columns: auto 180px 1fr;
  align-items: center;
  gap: 0;
  padding: 12px 20px;
  background: var(--bg-deep);
  border-bottom: 1px solid var(--border);
}

.command-strip__zone {
  display: flex;
  gap: 16px;
  align-items: center;
}

.command-strip__divider {
  width: 1px;
  height: 48px;
  background: linear-gradient(180deg, transparent, var(--border), transparent);
  margin: 0 20px;
}
```

### Responsive Strategy

| Breakpoint | Behavior |
|---|---|
| ≥1920px | Full 4-column grid as designed |
| 1440-1919px | Brain column narrows to 280px, charts stack 2+1 |
| 1024-1439px | 3-column: Brain moves to Tier 3 (tab/overlay), grid becomes `1fr 1fr 1fr` |
| 768-1023px | 2-column: Ops full-width top, Mesh + Tasks below, Analytics tabbed |
| <768px | Single column, Command Strip becomes 2-row stacked, all widgets full-width |

```css
@media (max-width: 1439px) {
  .dash-grid {
    grid-template-columns: repeat(3, 1fr);
    grid-template-areas:
      "status  status  status"
      "ops     ops     mesh"
      "tasks   tasks   charts"
      "timeline timeline timeline";
  }
  #brain-widget { display: none; } /* Moved to Brain tab */
}

@media (max-width: 1023px) {
  .dash-grid {
    grid-template-columns: 1fr 1fr;
    grid-template-areas:
      "status status"
      "ops    mesh"
      "tasks  tasks"
      "charts charts"
      "timeline timeline";
  }
}

@media (max-width: 767px) {
  .dash-grid {
    grid-template-columns: 1fr;
    grid-template-areas:
      "status" "ops" "mesh" "tasks" "charts" "timeline";
  }
}
```

### Estimated Scroll Reduction

```
Current:  ~3200px total height → requires 2.3 full scrolls on 1080p
Proposal: ~1100px total height → everything above fold on 1080p
           (Timeline strip may require ~100px scroll on compact displays)
Reduction: ~65% less scrolling
```

---

## Proposal B: "Command Center Triptych"

> Inspired by Bloomberg Terminal + Grafana — three functional zones with fixed sidebar

### Grid Structure

```
Fixed left sidebar (280px) + fluid center + fixed right sidebar (320px)
Center uses 2-row layout
```

```css
.dash-triptych {
  display: grid;
  grid-template-columns: 280px 1fr 320px;
  grid-template-rows: auto 1fr;
  grid-template-areas:
    "strip   strip   strip"
    "sidebar center  intel";
  height: calc(100vh - 56px); /* viewport minus header */
  overflow: hidden;
}

.dash-sidebar    { grid-area: sidebar; overflow-y: auto; }
.dash-center     { grid-area: center;  overflow-y: auto; }
.dash-intel      { grid-area: intel;   overflow-y: auto; }
```

### ASCII Layout (1920px viewport)

```
┌──────────────────────────────────────────────────────────────────────┐
│ HEADER: [Overview][Admin][Planner][Brain][IdeaJar]  Convergio   ⟳ ☰ │
├──────────────────────────────────────────────────────────────────────┤
│              COMMAND STRIP (compact, single-row)                      │
│ Active:3 │ Mesh:3/3 │ ◎Agents:5 │ Tokens:2.1M │ $4.20 │ ▓▓░ 82%   │
├──────────┬───────────────────────────────────────────┬───────────────┤
│          │                                           │               │
│ MISSION  │            CENTER STAGE                   │  INTEL PANEL  │
│ SIDEBAR  │                                           │               │
│          │  ┌─────────────────────────────────────┐  │ ┌───────────┐ │
│ ┌──────┐ │  │         TASK PIPELINE                │  │ │ Brain     │ │
│ │Active│ │  │  ┌────┬────────┬──────┬─────┬─────┐ │  │ │ (mini)    │ │
│ │Miss. │ │  │  │ ID │ Task   │Status│Plan │Model│ │  │ │           │ │
│ │      │ │  │  ├────┼────────┼──────┼─────┼─────┤ │  │ │  280px    │ │
│ │▸ 291 │ │  │  │ T1 │fix auth│▸exec │ 291 │claude-opus-4.6 │ │  │ │  tall     │ │
│ │▸ 289 │ │  │  │ T2 │add test│ pend │ 291 │codex│ │  │ │           │ │
│ │▸ 287 │ │  │  │ T3 │refactor│▸exec │ 289 │sonn.│ │  │ └───────────┘ │
│ │      │ │  │  └────┴────────┴──────┴─────┴─────┘ │  │               │
│ │ 24h: │ │  └─────────────────────────────────────┘  │ ┌───────────┐ │
│ │▪ 285✓│ │                                           │ │ Mesh Hub  │ │
│ │▪ 284✓│ │  ┌────────────┬────────────┬───────────┐  │ │ ┌───────┐ │ │
│ │▪ 283✓│ │  │ Task Dist  │ Cost/Model │ Token Burn│  │ │ │worker-1│ │ │
│ │      │ │  │    ◔       │   ▓▓▓     │   ╱╲      │  │ │ │CPU ▓▓ │ │ │
│ └──────┘ │  │   pie      │  stacked  │  ╱  ╲╱    │  │ │ │RAM ▓░ │ │ │
│          │  │   chart    │   bar     │  area      │  │ │ │↕ 12ms │ │ │
│ ┌──────┐ │  └────────────┴────────────┴───────────┘  │ │ ├───────┤ │ │
│ │Plan  │ │                                           │ │ │worker-2  │ │ │
│ │Kanban│ │  ┌─────────────────────────────────────┐  │ │ │CPU ▓░ │ │ │
│ │      │ │  │         PLAN TIMELINE                │  │ │ │RAM ▓▓ │ │ │
│ │Pipe  │ │  │ ◇ 291 ▓▓▓▓░░  ◇ 289 ▓▓▓▓▓▓░░      │  │ │ │↕ 8ms  │ │ │
│ │line→ │ │  │ ◇ 287 ▓▓░░░░  ◇ 285 ▓▓▓▓▓▓▓▓      │  │ │ ├───────┤ │ │
│ │Exec→ │ │  └─────────────────────────────────────┘  │ │ │linux-worker│ │ │
│ │Done→ │ │                                           │ │ └───────┘ │ │
│ │      │ │                                           │ │           │ │
│ └──────┘ │                                           │ ┌───────────┐ │
│          │                                           │ │ Activity  │ │
│ ┌──────┐ │                                           │ │ ▪ push..  │ │
│ │Histor│ │                                           │ │ ▪ PR #42  │ │
│ │ y    │ │                                           │ │ ▪ nightly │ │
│ │▪ P280│ │                                           │ │ ▪ deploy  │ │
│ │▪ P279│ │                                           │ │           │ │
│ └──────┘ │                                           │ └───────────┘ │
├──────────┴───────────────────────────────────────────┴───────────────┤
```

### Widget Grouping & Merges

| Current Widget(s) | Proposal B Treatment | Rationale |
|---|---|---|
| Active Missions + Last Missions + History | **Left sidebar stack**: Active (expandable) → Recent → History (scrollable) | Temporal narrative flows top-to-bottom |
| Plan Pipeline (Kanban) | **Left sidebar**: Compact vertical kanban (column per row, counts only) | Overview counts, click to expand full kanban overlay |
| Task Pipeline | **Center stage hero** — full-width table, maximum columns visible | This is THE work surface; deserves the most width |
| 3 charts | **Center stage row 2** — side-by-side triptych below task table | Visible without scroll, contextual to tasks above |
| Plan Timeline | **Center stage row 3** — Gantt bars | Relates to task table above |
| Mesh + Latency | **Right sidebar "Mesh Hub"** — vertically stacked node cards with inline latency | Infrastructure monitoring = always-visible sidebar |
| Brain | **Right sidebar mini** — 280px wide, 280px tall compact visualization | Signature feature preserved but contained |
| GitHub + Nightly + Events | **Right sidebar "Activity"** — unified scrollable feed | Low-priority streams in peripheral vision |
| Idea Jar | **Nav tab only** — removed from grid | On-demand access |

### Visual Hierarchy

```
TIER 1 — FOCAL (center stage, immediate attention):
  → Task Pipeline table (the active work)
  → Command Strip KPIs (system pulse)

TIER 2 — PERIPHERAL (sidebars, always visible, glanceable):
  → Mission Sidebar (left) — what's running
  → Mesh Hub (right) — system health
  → Analytics Triptych (center row 2)

TIER 3 — AMBIENT (lower priority, scrollable within zones):
  → Brain mini-viz (right sidebar)
  → Activity Feed (right sidebar bottom)
  → History (left sidebar bottom)
  → Plan Timeline (center bottom)
```

### KPI Bar Redesign: "Ticker Strip"

Bloomberg-inspired single-line ticker:

```
┌──────────────────────────────────────────────────────────────────────┐
│ ● ACTIVE 3  │  ⬡ MESH 3/3  │  ◎ AGENTS 5  │  ⟐ 2.1M tkn  │  $ 4.20  │  ▓▓▓▓░░ 82%  │  ⚠ 0 blocked │
└──────────────────────────────────────────────────────────────────────┘
```

**Design**:
- Single horizontal strip, 40px tall (saved 100px vs current)
- Each metric: icon + label + value, separated by thin dividers
- Color-coded values: green (healthy), amber (warning), red (critical)
- No gauges in strip — Agents gauge moved to left sidebar header
- Sparkline micro-charts (40×16px) on hover per metric

```css
.ticker-strip {
  display: flex;
  align-items: center;
  height: 40px;
  padding: 0 20px;
  gap: 0;
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border);
  font-family: var(--font-mono);
  font-size: 13px;
}

.ticker-strip__item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 16px;
  border-right: 1px solid var(--border);
  white-space: nowrap;
}

.ticker-strip__value {
  font-weight: 700;
  font-size: 15px;
  color: var(--accent);
}

.ticker-strip__item:hover .ticker-sparkline {
  opacity: 1;
  transform: translateY(0);
}
```

### Responsive Strategy

| Breakpoint | Behavior |
|---|---|
| ≥1920px | Full triptych: 280px + fluid + 320px |
| 1440-1919px | Sidebars shrink: 240px + fluid + 280px |
| 1024-1439px | Right sidebar collapses to icon rail (48px), click to expand overlay |
| 768-1023px | Both sidebars collapse to icon rails, center is full width |
| <768px | No sidebars, tab navigation between Mission/Tasks/Intel views |

```css
@media (max-width: 1439px) {
  .dash-triptych {
    grid-template-columns: 240px 1fr 48px;
  }
  .dash-intel {
    overflow: visible; /* allow overlay */
  }
  .dash-intel:hover,
  .dash-intel.expanded {
    width: 320px;
    position: absolute;
    right: 0;
    box-shadow: -4px 0 24px rgba(0,0,0,0.5);
  }
}

@media (max-width: 1023px) {
  .dash-triptych {
    grid-template-columns: 48px 1fr 48px;
  }
}

@media (max-width: 767px) {
  .dash-triptych {
    grid-template-columns: 1fr;
    grid-template-areas: "strip" "center";
  }
  .dash-sidebar, .dash-intel {
    display: none; /* use tab nav */
  }
}
```

### Estimated Scroll Reduction

```
Each zone scrolls independently within viewport height.
Center stage: ~700px content fits in ~884px available → NO scroll needed
Left sidebar: ~900px content in 884px → minimal scroll for History only
Right sidebar: ~1000px → scroll within contained panel

Net effect: Zero page-level scroll. Each panel scrolls independently.
Reduction: 100% of page-level scrolling eliminated
```

---

## Proposal C: "Adaptive Mosaic"

> Inspired by Grafana + i3wm tiling — responsive mosaic with collapsible panels

### Grid Structure

```
CSS Grid with named areas, 12-column base grid
Mosaic tiles fill available space with priority-based sizing
```

```css
.dash-mosaic {
  display: grid;
  grid-template-columns: repeat(12, 1fr);
  grid-template-rows: auto minmax(200px, 1fr) minmax(180px, auto) auto;
  grid-template-areas:
    "strip  strip  strip  strip  strip  strip  strip  strip  strip  strip  strip  strip"
    "ops    ops    ops    tasks  tasks  tasks  tasks  tasks  mesh   mesh   mesh   mesh"
    "charts charts charts charts brain  brain  brain  brain  brain  feed   feed   feed"
    "tline  tline  tline  tline  tline  tline  tline  tline  tline  tline  tline  tline";
  gap: 10px;
  padding: 12px 16px;
  height: calc(100vh - 56px);
}
```

### ASCII Layout (1920px viewport)

```
┌──────────────────────────────────────────────────────────────────────┐
│ HEADER: [Overview][Admin][Planner][Brain][IdeaJar]  Convergio   ⟳ ☰ │
├──────────────────────────────────────────────────────────────────────┤
│                    COMMAND MOSAIC STRIP                               │
│  ┌────────────────────┐  ◎ 5 agents  ┌──────────────────────────┐   │
│  │ 3 active │ 3/3 mesh│  running     │ 2.1M tkn │ $4.20 │ 0 ⚠  │   │
│  │ 47 total │ 12ms avg│             │ 80K today │ ▓▓░82%│      │   │
│  └────────────────────┘              └──────────────────────────┘   │
├─────────────────┬──────────────────────────────┬────────────────────┤
│                 │                              │                    │
│  OPS PANEL      │    TASK COMMAND TABLE         │  MESH MONITOR      │
│  (3 cols)       │    (5 cols)                   │  (4 cols)          │
│                 │                              │                    │
│ ┌─────────────┐ │ ┌────────────────────────┐   │ ┌────────────────┐ │
│ │ ▸ Plan-291  │ │ │ID  Task      Stat Plan │   │ │ ┌──┐ ┌──┐ ┌──┐│ │
│ │   3/8 tasks │ │ │T1  fix auth  ▸exe 291  │   │ │ │m1│ │m3│ │om││ │
│ │   wave 2    │ │ │T2  add tests pend 291  │   │ │ │  │ │  │ │  ││ │
│ │ ▸ Plan-289  │ │ │T3  refactor  ▸exe 289  │   │ │ │▓▓│ │▓░│ │▓▓││ │
│ │   7/7 tasks │ │ │T4  migrate   pend 289  │   │ │ │12│ │8 │ │15││ │
│ │   wave 3    │ │ │T5  deploy    wait 287  │   │ │ │ms│ │ms│ │ms││ │
│ │ ▸ Plan-287  │ │ │T6  validate  pend 287  │   │ │ └──┘ └──┘ └──┘│ │
│ │   1/4 tasks │ │ └────────────────────────┘   │ │  Hub topology   │ │
│ │             │ │                              │ │  ╱    │    ╲    │ │
│ │ Recent:     │ │                              │ │ m1──coord──om  │ │
│ │ ▪ 285 ✓ 2h │ │                              │ │       │        │ │
│ │ ▪ 284 ✓ 5h │ │                              │ │      m3        │ │
│ └─────────────┘ │                              │ └────────────────┘ │
├─────────────────┼──────────────────────────────┼────────────────────┤
│                 │                              │                    │
│ ANALYTICS       │    AUGMENTED BRAIN           │  ACTIVITY FEED     │
│ (3 cols)        │    (5 cols)                  │  (3 cols)          │
│                 │                              │                    │
│ ┌─────────────┐ │ ┌──────────────────────────┐ │ ┌────────────────┐ │
│ │ TaskDist ◔  │ │ │                          │ │ │ ▪ push main    │ │
│ │             │ │ │    Neural Network         │ │ │ ▪ PR #42 merge│ │
│ │ Cost    ▓▓▓ │ │ │    (compact, ~220px)      │ │ │ ▪ nightly ✓   │ │
│ │             │ │ │                          │ │ │ ▪ deploy prod  │ │
│ │ Burn   ╱╲  │ │ │    Nodes as constellation │ │ │ ▪ issue #89   │ │
│ │       ╱  ╲ │ │ │    Click to expand full   │ │ │               │ │
│ └─────────────┘ │ └──────────────────────────┘ │ └────────────────┘ │
├─────────────────┴──────────────────────────────┴────────────────────┤
│                    TIMELINE RAIL (collapsible)                       │
│  ◇ 291 ▓▓▓▓░░░  ◇ 289 ▓▓▓▓▓▓▓░  ◇ 287 ▓▓░░░░  │ Kanban ▸│ ▼    │
└──────────────────────────────────────────────────────────────────────┘
```

### Widget Grouping & Merges

| Current Widget(s) | Proposal C Treatment | Rationale |
|---|---|---|
| Active Missions + Last Missions | **Ops Panel** — mission cards with inline progress bars, recent list below | Compact vertical list, each mission is one dense line |
| Task Pipeline | **Task Command Table** — hero table, maximum width (5/12 cols) | Central work surface, needs room for columns |
| Mesh Network + Latency | **Mesh Monitor** — node columns with CPU/RAM bars + latency, mini topology diagram below | Single composite: cards + topology in one widget |
| 3 charts | **Analytics Stack** — vertically stacked mini-charts (3 cols wide) | Narrow but full-height, works as sparkline-style |
| Brain | **Compact Brain** — 220px tall constellation mode, expand to overlay on click | Visible but contained; click → full-screen overlay |
| GitHub + Nightly + Events | **Activity Feed** — unified chronological feed with type badges | Single stream with filters |
| Plan Timeline + Kanban | **Timeline Rail** — bottom strip, collapsible, with kanban toggle | Persistent but minimal; expand when needed |
| History | **Inside Ops Panel** — scrollable below active missions | Temporal flow: active → recent → history |
| Idea Jar | **Nav tab** (removed from grid) | On-demand access |

### Visual Hierarchy

```
TIER 1 — HERO (largest grid areas, highest contrast):
  → Task Command Table [5 cols, row 2] — THE primary workspace
  → Command Strip KPIs [full width, row 1] — system vitals

TIER 2 — SUPPORTING (medium areas, standard contrast):
  → Ops Panel [3 cols, row 2] — mission context for tasks
  → Mesh Monitor [4 cols, row 2] — infrastructure health

TIER 3 — CONTEXTUAL (smaller areas, reduced opacity/contrast):
  → Analytics Stack [3 cols, row 3] — trends
  → Augmented Brain [5 cols, row 3] — visualization
  → Activity Feed [3 cols, row 3] — events

TIER 4 — ON-DEMAND (collapsible, minimal footprint):
  → Timeline Rail [12 cols, row 4] — plan progress
```

### KPI Bar Redesign: "Mosaic Strip"

Two-cluster layout with central gauge:

```
┌──────────────────────────────────────────────────────────────────┐
│ ┌─────────────────────┐          ┌───────────────────────────┐  │
│ │  3 active  │  3/3   │  ◎  5   │  2.1M   │ $4.20 │  0 ⚠   │  │
│ │  47 total  │  mesh  │ agents  │  tokens  │ cost  │ blocked │  │
│ │            │ 12ms ↕ │ running │  80K/day │ ▓▓░   │         │  │
│ └─────────────────────┘          └───────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

**Design**:
- **Left cluster (glass card)**: Plans + Mesh — "what's alive?"
- **Center hero**: Agent gauge (120×120px) — "the heartbeat"
- **Right cluster (glass card)**: Tokens, Cost, Progress, Blocked — "the cost of work"
- Height: ~80px (saved 60px vs current)
- Clusters use glassmorphism (backdrop-filter: blur) for depth

```css
.mosaic-strip {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 20px;
  height: 80px;
  gap: 20px;
}

.mosaic-strip__cluster {
  display: flex;
  gap: 20px;
  padding: 10px 20px;
  background: rgba(26, 26, 26, 0.7);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(42, 42, 42, 0.5);
  border-radius: 12px;
}

.mosaic-strip__hero {
  flex: 0 0 120px;
  display: flex;
  align-items: center;
  justify-content: center;
}
```

### Responsive Strategy

| Breakpoint | Behavior |
|---|---|
| ≥1920px | Full 12-col mosaic, all 4 rows visible |
| 1440-1919px | Same grid, columns compress proportionally (fr units handle it) |
| 1024-1439px | Row 3 collapses: Brain → full overlay, Analytics + Feed become tabs below Task table |
| 768-1023px | 2-row layout: Strip + scrollable card stack. Each "tile" is full-width, priority ordered |
| <768px | Single column, priority-sorted: Strip → Tasks → Ops → Mesh → (rest as tabs) |

**Key responsive feature — Tile Priority Sorting**:

```css
/* Mobile: flex column with order-based priority */
@media (max-width: 767px) {
  .dash-mosaic {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  [data-grid-area="strip"]  { order: 0; }
  [data-grid-area="tasks"]  { order: 1; }
  [data-grid-area="ops"]    { order: 2; }
  [data-grid-area="mesh"]   { order: 3; }
  [data-grid-area="charts"] { order: 4; }
  [data-grid-area="brain"]  { order: 5; display: none; } /* tab access */
  [data-grid-area="feed"]   { order: 6; }
  [data-grid-area="tline"]  { order: 7; }
}
```

### Estimated Scroll Reduction

```
Row 1 (Strip):     80px
Row 2 (Ops/Tasks/Mesh): ~400px (minmax 200px, 1fr)
Row 3 (Charts/Brain/Feed): ~260px (minmax 180px, auto)
Row 4 (Timeline):  60px (collapsed) / 120px (expanded)
TOTAL:             ~800px — fits in 884px available

Reduction: 100% — no page scroll on 1080p
           Row 4 auto-collapses if needed
```

---

## Comparative Analysis

| Criterion | A: Mission Control | B: Triptych | C: Mosaic |
|---|---|---|---|
| **Page scroll** | ~100px overflow | Zero (independent panels) | Zero (viewport-locked) |
| **Task table width** | 2/4 cols = ~50% viewport | Fluid center = ~55% viewport | 5/12 cols = ~42% viewport |
| **Above-fold widgets** | 6 of 8 groups | All (panels scroll internally) | All (rows fill viewport) |
| **Brain visibility** | Right column, 420px | Right sidebar, 280px mini | Row 3, 220px compact |
| **Learning curve** | Low (grid is intuitive) | Medium (3 panels + scrolling) | Low (everything visible) |
| **Information density** | High | Very High | Highest |
| **Responsive quality** | Good (column drop) | Excellent (sidebar collapse) | Excellent (tile priority) |
| **Implementation effort** | Medium (grid-template-areas) | High (independent scroll zones) | Medium (grid + order) |
| **Drag-drop compat** | ✅ (grid areas) | ⚠️ (sidebar constraints) | ✅ (tiles reorderable) |
| **Widget chrome saved** | 5 widgets merged | 6 widgets merged | 6 widgets merged |
| **KPI bar height** | ~100px (3-zone) | ~40px (ticker) | ~80px (2-cluster) |

---

## Recommendation

### Primary: **Proposal B — Command Center Triptych**

**Why**: It eliminates 100% of page-level scrolling by giving each zone its own scroll context. The Task Pipeline gets maximum width in the fluid center. The sidebar pattern is familiar from IDEs and dashboards (VS Code, Grafana, Bloomberg). The Brain visualization fits naturally in the right intel panel without competing for space.

### Modifications to Apply from Other Proposals

1. **From Proposal A**: Use the "Command Strip" KPI bar (3-zone, ~100px) instead of the 40px ticker. The ticker is too compressed for a dashboard where KPIs are primary.
2. **From Proposal C**: Add the tile priority sorting for mobile responsive — cleaner than sidebar collapsing on small screens.
3. **From Proposal A**: Keep the Analytics Triptych as a horizontal row in center stage (not stacked vertically as in B's original).

### Hybrid Implementation Spec

```css
.dash-hybrid {
  display: grid;
  grid-template-columns: 280px 1fr 320px;
  grid-template-rows: auto auto 1fr;
  grid-template-areas:
    "header header header"
    "strip  strip  strip"
    "sidebar center intel";
  height: 100vh;
  overflow: hidden;
}

.dash-center {
  display: grid;
  grid-template-rows: 1fr auto auto;
  gap: 10px;
  padding: 10px;
  overflow-y: auto;
}

/* Center internal layout */
.dash-center__tasks    { /* Task Pipeline - hero */ }
.dash-center__analytics { 
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 10px;
}
.dash-center__timeline { /* Collapsible Gantt */ }
```

---

## Interaction Design Notes

### Progressive Disclosure Patterns

| Widget | Collapsed State | Expanded State | Trigger |
|---|---|---|---|
| Mission Card | Plan ID + progress bar + status badge | Full task list, wave details, logs | Click card |
| Mesh Node | Name + CPU/RAM bars + latency | Full metrics, sync history, actions | Click node |
| Brain | 280px mini constellation | Full-screen overlay with controls | Double-click / expand button |
| Analytics Chart | 180px compact chart | Full-width modal with time range controls | Click chart |
| Activity Feed Item | One-line summary | Full diff/event details | Click item |
| Timeline Rail | Collapsed strip (60px) | Expanded Gantt (200px) | Click expand chevron |
| Task Row | Summary line | Side panel with task details, logs, test results | Click row |
| KPI Metric | Value + label | Sparkline history + trend + alert thresholds | Hover |

### Keyboard Navigation (WCAG 2.1 AA)

```
Tab:        Move between zones (sidebar → center → intel)
Arrow keys: Navigate within zone (up/down in lists, left/right in tabs)
Enter:      Expand/select item
Escape:     Collapse overlay / return to overview
1-9:        Quick-jump to zone (1=KPIs, 2=Missions, 3=Tasks, etc.)
/:          Focus search/filter
?:          Show keyboard shortcut overlay
```

### Accessibility Specifications

- All color-coded states also have icon + text alternatives
- Gauge values announced via `aria-valuenow` / `aria-valuemin` / `aria-valuemax`
- Brain visualization has `aria-label` with session count and status summary
- Reduced motion: `@media (prefers-reduced-motion: reduce)` disables all animations including breathing glow, status pulses
- Focus indicators: 2px solid var(--accent) outline with 2px offset
- Minimum touch target: 44×44px for all interactive elements

---

## Next Steps

1. **Review**: Engineering team reviews proposals, selects primary direction
2. **Prototype**: Build static HTML/CSS prototype of hybrid layout (no JS) — 2 days
3. **User test**: 3 internal users test with think-aloud protocol — 1 day
4. **Implement**: Migrate current grid to new layout, preserve widget-drag persistence — 3-5 days
5. **Polish**: Responsive testing across 1920/1440/1024/768/375 breakpoints — 2 days

---

*Sara — Senior UX/UI Designer*
*Maranello Luce Design System v24*
