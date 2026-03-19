# MLD Component Request: mn-app-shell layout="control-center"

**Status:** Proposed | **Target:** MaranelloLuceDesign | **Date:** 2026-03-19

## Problem

The Mission Control GUI requires a compact, mobile-inspired layout distinct from the existing dashboard shell. The current `mn-app-shell` assumes a sidebar-based desktop layout. Mission Control needs a fixed header + tab bar chrome with content filling the remaining viewport, constrained to 480px max-width for a focused, phone-like experience.

## Proposed API

```html
<mn-app-shell layout="control-center">
  <mn-app-header slot="header">Mission Control</mn-app-header>
  <mn-tab-bar slot="tabs">
    <mn-tab value="overview" icon="dashboard" selected>Overview</mn-tab>
    <mn-tab value="mesh" icon="network">Mesh</mn-tab>
    <mn-tab value="plans" icon="list">Plans</mn-tab>
    <mn-tab value="chat" icon="chat">Chat</mn-tab>
  </mn-tab-bar>
  <div slot="content">
    <!-- Active tab content renders here -->
  </div>
</mn-app-shell>
```

## Props

| Prop | Type | Default | Description |
|---|---|---|---|
| `layout` | `'default' \| 'control-center'` | `'default'` | Shell layout variant |
| `max-width` | `string` | `'480px'` | Max content width (control-center only) |
| `header-height` | `string` | `'40px'` | Fixed header height |
| `tab-bar-height` | `string` | `'32px'` | Tab bar height |

## Events

| Event | Detail | Description |
|---|---|---|
| `mn-tab-change` | `{ value: string, previousValue: string }` | Fired when active tab changes |
| `mn-layout-ready` | `{ layout: string, contentHeight: number }` | Fired after layout calculation completes |

## Layout Rules (control-center variant)

| Region | Sizing |
|---|---|
| Header | Fixed 40px, full width, pinned top |
| Tab bar | Fixed 32px, below header, full width |
| Content | `calc(100vh - 72px)`, scrollable, centered |
| Sidebar | Hidden (not rendered) |
| Max-width | 480px, centered with `margin: 0 auto` |

## Usage Example

```js
const shell = document.querySelector('mn-app-shell');
shell.setAttribute('layout', 'control-center');

shell.addEventListener('mn-tab-change', (e) => {
  const { value } = e.detail;
  loadTabContent(value);
});
```

## Accessibility Notes

- Tab bar implements `role="tablist"` with `role="tab"` children
- Arrow keys navigate between tabs; Enter/Space activates
- Active tab uses `aria-selected="true"`, content panel uses `aria-labelledby`
- Header landmark: `role="banner"`
- Content region: `role="main"` with `aria-live="polite"` for tab switches
- Focus moves to content panel on tab change
- Minimum touch target 44x44px for tab items
- Supports `prefers-reduced-motion` for tab transitions
