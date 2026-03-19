# MLD Component Request: mn-notification-feed

**Status:** Proposed | **Target:** MaranelloLuceDesign | **Date:** 2026-03-19

## Problem

Mission Control needs a real-time notification feed showing mesh events, plan updates, and system alerts. Users must filter by severity, mark items as read, and dismiss them. The feed connects to SSE/WebSocket data sources and caps at 50 items to bound memory and DOM size.

## Proposed API

```html
<mn-notification-feed
  src="ws://localhost:8420/ws/notifications"
  max-items="50"
  filter="all"
>
  <mn-notification-badge slot="badge"></mn-notification-badge>
</mn-notification-feed>
```

## Props

| Prop | Type | Default | Description |
|---|---|---|---|
| `src` | `string` | required | SSE or WebSocket endpoint URL |
| `src-type` | `'sse' \| 'ws'` | `'ws'` | Data source transport |
| `max-items` | `number` | `50` | Maximum retained notifications |
| `filter` | `'all' \| 'info' \| 'warning' \| 'error' \| 'critical'` | `'all'` | Active type filter |
| `auto-reconnect` | `boolean` | `true` | Reconnect on connection loss |
| `reconnect-interval` | `number` | `3000` | Reconnect delay in ms |

## Events

| Event | Detail | Description |
|---|---|---|
| `mn-notification-receive` | `{ notification: Notification }` | New notification received |
| `mn-notification-read` | `{ id: string }` | Notification marked as read |
| `mn-notification-dismiss` | `{ id: string }` | Notification dismissed |
| `mn-notification-filter` | `{ filter: string }` | Filter changed |
| `mn-feed-overflow` | `{ dropped: number }` | Items dropped due to max-items cap |
| `mn-feed-connection` | `{ status: 'connected' \| 'disconnected' \| 'reconnecting' }` | Connection state change |

## Notification Shape

```ts
interface Notification {
  id: string;
  type: 'info' | 'warning' | 'error' | 'critical';
  title: string;
  message: string;
  timestamp: string;       // ISO 8601
  source?: string;         // e.g. "mesh", "plan-db", "daemon"
  read: boolean;
  dismissable: boolean;
  action?: {
    label: string;
    href?: string;
    handler?: string;      // registered action name
  };
}
```

## Methods

| Method | Signature | Description |
|---|---|---|
| `markRead` | `(id: string) => void` | Mark single notification as read |
| `markAllRead` | `() => void` | Mark all notifications as read |
| `dismiss` | `(id: string) => void` | Remove notification from feed |
| `clearAll` | `() => void` | Remove all notifications |
| `getUnreadCount` | `() => number` | Return unread count |

## Usage Example

```js
const feed = document.querySelector('mn-notification-feed');
feed.src = 'ws://localhost:8420/ws/notifications';
feed.filter = 'error';

feed.addEventListener('mn-notification-receive', (e) => {
  const { notification } = e.detail;
  if (notification.type === 'critical') {
    showUrgentBanner(notification);
  }
});

// Badge reflects unread count automatically
const badge = feed.querySelector('mn-notification-badge');
// badge.count updates via internal binding
```

## Accessibility Notes

- Feed container: `role="log"` with `aria-live="polite"` (`aria-live="assertive"` for critical)
- Each notification: `role="article"` with `aria-label` combining type and title
- Unread items marked with `aria-current="true"`
- Badge uses `aria-label="N unread notifications"`
- Filter controls: `role="radiogroup"` with `role="radio"` options
- Dismiss button: `aria-label="Dismiss notification: {title}"`
- Keyboard: Tab to navigate items, Delete to dismiss, Enter to activate action
- Screen reader announces new critical/error notifications immediately
- Supports `prefers-reduced-motion` for slide-in animations
