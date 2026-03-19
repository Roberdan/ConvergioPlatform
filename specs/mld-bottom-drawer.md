# MLD Component Request: Bottom Drawer CSS Pattern

**Status:** Proposed | **Target:** MaranelloLuceDesign | **Date:** 2026-03-19

## Problem

Mission Control needs a VS Code-style resizable bottom panel for logs, terminal output, and detail views. The panel slides up from the bottom, has a drag handle for resizing, respects min/max height constraints, and layers correctly above content but below modals.

## Proposed API

```html
<mn-bottom-drawer
  min-height="120"
  max-height="60vh"
  default-height="240"
  open
>
  <span slot="title">Logs</span>
  <div slot="content">
    <!-- Scrollable panel content -->
  </div>
</mn-bottom-drawer>
```

## Props

| Prop | Type | Default | Description |
|---|---|---|---|
| `open` | `boolean` | `false` | Whether the drawer is visible |
| `min-height` | `string` | `'120px'` | Minimum panel height |
| `max-height` | `string` | `'60vh'` | Maximum panel height |
| `default-height` | `string` | `'240px'` | Initial panel height |
| `resizable` | `boolean` | `true` | Enable drag-to-resize handle |
| `animation-duration` | `string` | `'200ms'` | Slide animation duration |

## Events

| Event | Detail | Description |
|---|---|---|
| `mn-drawer-open` | `{ height: number }` | Drawer opened |
| `mn-drawer-close` | `{}` | Drawer closed |
| `mn-drawer-resize` | `{ height: number, percentage: number }` | Drag handle moved |
| `mn-drawer-resize-end` | `{ height: number }` | Drag released |

## CSS Custom Properties

| Property | Default | Description |
|---|---|---|
| `--mn-drawer-bg` | `var(--mn-surface-1)` | Panel background |
| `--mn-drawer-border` | `var(--mn-border-color)` | Top border color |
| `--mn-drawer-handle-color` | `var(--mn-text-tertiary)` | Drag handle color |
| `--mn-drawer-handle-hover` | `var(--mn-text-secondary)` | Drag handle hover color |
| `--mn-drawer-z-index` | `100` | Panel z-index layer |
| `--mn-drawer-shadow` | `0 -2px 8px rgba(0,0,0,0.1)` | Top shadow |
| `--mn-drawer-radius` | `8px 8px 0 0` | Top border radius |

## Z-Index Layering

| Layer | Z-Index | Element |
|---|---|---|
| Content | 1 | Main content area |
| Bottom drawer | 100 | This component |
| Overlays/Popovers | 200 | Dropdowns, tooltips |
| Modals | 300 | Modal dialogs |
| Toasts | 400 | Toast notifications |

## Drag Handle Behavior

- 4px tall hit area, centered 32px wide pill indicator
- Cursor changes to `row-resize` on hover
- Drag snaps to min-height or max-height at boundaries
- Double-click toggles between default-height and max-height
- Persists last height to `localStorage` key `mn-drawer-height`

## Usage Example

```js
const drawer = document.querySelector('mn-bottom-drawer');

// Toggle open/close
drawer.open = !drawer.open;

// Listen for resize
drawer.addEventListener('mn-drawer-resize-end', (e) => {
  const { height } = e.detail;
  adjustContentArea(height);
});

// Programmatic resize
drawer.style.setProperty('--mn-drawer-z-index', '150');
```

## Integration with mn-app-shell

When used inside `mn-app-shell layout="control-center"`, the shell content area shrinks to accommodate the drawer. The drawer anchors to the bottom of the content region, not the viewport.

```html
<mn-app-shell layout="control-center">
  <div slot="content">
    <main>...</main>
    <mn-bottom-drawer open>
      <span slot="title">Terminal</span>
      <div slot="content">...</div>
    </mn-bottom-drawer>
  </div>
</mn-app-shell>
```

## Accessibility Notes

- Drag handle: `role="separator"` with `aria-orientation="horizontal"` and `aria-valuenow`
- Keyboard resize: Arrow Up/Down moves by 10px, Shift+Arrow by 50px
- Panel: `role="region"` with `aria-label` from title slot
- Toggle button (if present): `aria-expanded` reflects open state
- Focus trapped within drawer when opened via keyboard
- Slide animation respects `prefers-reduced-motion: reduce`
- Minimum height ensures content remains readable at 200% zoom
