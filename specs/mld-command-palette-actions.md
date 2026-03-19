# MLD Component Request: mn-command-palette registerAction API

**Status:** Proposed | **Target:** MaranelloLuceDesign | **Date:** 2026-03-19

## Problem

The existing `mn-command-palette` component supports built-in navigation commands but lacks an API for domain-specific actions. Mission Control needs to register mesh operations, plan management, and LLM actions into the palette at runtime without modifying the component source.

## Proposed API

```js
const palette = document.querySelector('mn-command-palette');

palette.registerAction({
  category: 'Mesh',
  label: 'Sync All Nodes',
  handler: async () => {
    await fetch('/api/v1/mesh/sync', { method: 'POST' });
  },
  shortcut: 'Ctrl+Shift+S',
  icon: 'sync',
  description: 'Trigger full mesh synchronization across all peers',
  when: () => meshStatus.connected,
});
```

## Method Signatures

| Method | Signature | Description |
|---|---|---|
| `registerAction` | `(action: PaletteAction) => string` | Register action, returns generated ID |
| `registerActions` | `(actions: PaletteAction[]) => string[]` | Bulk register, returns IDs |
| `unregisterAction` | `(id: string) => boolean` | Remove action by ID |
| `unregisterCategory` | `(category: string) => number` | Remove all actions in category, returns count |
| `getActions` | `(category?: string) => PaletteAction[]` | List registered actions |
| `executeAction` | `(id: string) => Promise<void>` | Programmatically trigger an action |

## PaletteAction Interface

```ts
interface PaletteAction {
  category: string;           // Grouping label (e.g. "Mesh", "Plans", "LLM")
  label: string;              // Display name
  handler: () => void | Promise<void>;  // Execution callback
  shortcut?: string;          // Keyboard shortcut (e.g. "Ctrl+Shift+S")
  icon?: string;              // MLD icon name
  description?: string;       // Secondary text shown in palette
  when?: () => boolean;       // Condition for visibility
  priority?: number;          // Sort order within category (lower = higher)
}
```

## Events

| Event | Detail | Description |
|---|---|---|
| `mn-action-register` | `{ id: string, category: string, label: string }` | Action registered |
| `mn-action-unregister` | `{ id: string }` | Action removed |
| `mn-action-execute` | `{ id: string, label: string, duration: number }` | Action executed (with timing) |
| `mn-action-error` | `{ id: string, error: Error }` | Action handler threw |

## Shortcut Format

Shortcuts follow the pattern: `Modifier+Key` where modifiers are `Ctrl`, `Shift`, `Alt`, `Meta`.

| Platform | `Ctrl` maps to | `Meta` maps to |
|---|---|---|
| macOS | `Control` | `Cmd` |
| Windows/Linux | `Ctrl` | `Win` |

Conflicts with built-in browser/OS shortcuts are silently ignored with a `console.warn`.

## Usage Example

```js
const palette = document.querySelector('mn-command-palette');

// Register mesh actions
const meshActions = [
  {
    category: 'Mesh',
    label: 'Sync All Nodes',
    handler: () => fetch('/api/v1/mesh/sync', { method: 'POST' }),
    shortcut: 'Ctrl+Shift+S',
    icon: 'sync',
  },
  {
    category: 'Mesh',
    label: 'Show Node Health',
    handler: () => navigateTo('/mesh/health'),
    shortcut: 'Ctrl+Shift+H',
    icon: 'heartbeat',
  },
];

const ids = palette.registerActions(meshActions);

// Conditional action (only when LLM is running)
palette.registerAction({
  category: 'LLM',
  label: 'Query Local Model',
  handler: () => openLLMPrompt(),
  when: () => llmService.isRunning(),
  shortcut: 'Ctrl+Shift+L',
});

// Clean up on view destroy
palette.unregisterCategory('Mesh');

// Handle errors
palette.addEventListener('mn-action-error', (e) => {
  const { id, error } = e.detail;
  console.warn(`Action ${id} failed:`, error.message);
  showToast(`Action failed: ${error.message}`, 'error');
});
```

## Palette Rendering

Registered actions appear grouped by category. Within each category, actions sort by `priority` (ascending), then alphabetically. The search input fuzzy-matches against `label`, `description`, and `category`.

```
 ───────────────────────────────
│ > Search actions...           │
│───────────────────────────────│
│ Mesh                          │
│   ⟳ Sync All Nodes   ^⇧S     │
│   ♥ Show Node Health  ^⇧H    │
│ LLM                          │
│   ◈ Query Local Model ^⇧L    │
│ Plans                         │
│   ▶ Execute Next Task ^⇧E    │
 ───────────────────────────────
```

## Accessibility Notes

- Palette opens with `Ctrl+K` or `Cmd+K`, traps focus, closes on `Escape`
- Action list: `role="listbox"` with `role="option"` items
- Category headers: `role="group"` with `aria-label`
- Shortcut displayed with `aria-keyshortcuts` attribute
- Arrow keys navigate items; Enter executes selected action
- Disabled actions (when `when()` returns false): `aria-disabled="true"`, skipped in keyboard navigation
- Search input: `role="combobox"` with `aria-autocomplete="list"`
- Action execution result announced via `aria-live="polite"` region
- All icon-only elements have `aria-label` fallbacks
