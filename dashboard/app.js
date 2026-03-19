// app.js — Convergio Control Room orchestrator
// Wires Maranello runtime, API clients, reactive store, and view modules.
import * as api from './lib/api-core.js';
import * as store from './lib/store.js';
import { connectDashboardWS } from './lib/ws.js';

const REFRESH_INTERVAL_MS = 16000;
const DEFAULT_VIEW = 'overview';

// View factories — lazy-loaded via dynamic import
const VIEW_MODULES = {
  overview: () => import('./views/overview.js'),
  plans: () => import('./views/plans.js'),
  mesh: () => import('./views/mesh.js'),
  brain: () => import('./views/brain.js'),
  ideas: () => import('./views/ideas.js'),
  ipc: () => import('./views/ipc.js'),
  admin: () => import('./views/admin.js'),
  terminal: () => import('./views/terminal.js'),
};

// Capitalise view id for display
function viewTitle(id) {
  return id.charAt(0).toUpperCase() + id.slice(1);
}

// Navigation state tracked here to avoid stale closures
let activeViewId = null;
let orch = null;

// ── Hash-based routing ──────────────────────────────────────────────

function viewFromHash() {
  const raw = location.hash.replace(/^#\/?/, '').split('/')[0];
  return VIEW_MODULES[raw] ? raw : null;
}

function setHash(viewId) {
  if (location.hash !== `#${viewId}`) {
    history.pushState(null, '', `#${viewId}`);
  }
}

function handleHashChange() {
  const viewId = viewFromHash();
  if (viewId && viewId !== activeViewId) {
    activateView(viewId);
  }
}

// ── View activation ─────────────────────────────────────────────────

function activateView(viewId) {
  if (!VIEW_MODULES[viewId] || !orch) return;
  activeViewId = viewId;
  setHash(viewId);
  orch.open(viewId, 'page');
  highlightNavLink(viewId);
}

function highlightNavLink(viewId) {
  document.querySelectorAll('[data-view]').forEach((link) => {
    link.classList.toggle('mn-sidebar__link--active', link.dataset.view === viewId);
  });
}

// ── Registration ────────────────────────────────────────────────────

function registerViews(registry) {
  for (const [id, loader] of Object.entries(VIEW_MODULES)) {
    registry.register({
      id,
      title: viewTitle(id),
      defaultPlacement: 'page',
      factory: async (container) => {
        const mod = await loader();
        return mod.default(container, { api, store });
      },
    });
  }

  // Plan-detail opens as a side panel (cross-view navigation)
  registry.register({
    id: 'plan-detail',
    title: 'Plan Detail',
    defaultPlacement: 'side-panel',
    factory: async (container, data) => {
      const mod = await import('./views/plans.js');
      return mod.planDetail(container, data, { api, store });
    },
  });
}

// ── Sidebar navigation ──────────────────────────────────────────────

function bindSidebarNav() {
  document.querySelectorAll('[data-view]').forEach((link) => {
    link.addEventListener('click', (e) => {
      e.preventDefault();
      const viewId = link.dataset.view;
      if (VIEW_MODULES[viewId]) activateView(viewId);
    });
  });
}

// ── Command palette ─────────────────────────────────────────────────

function bindCommandPalette() {
  const palette = document.getElementById('cmd-palette');
  if (!palette) return;

  palette.items = JSON.stringify(
    Object.keys(VIEW_MODULES).map((id) => ({
      text: viewTitle(id),
      group: 'Navigation',
    }))
  );

  palette.addEventListener('mn-select', (e) => {
    const viewId = e.detail.item.text.toLowerCase();
    if (VIEW_MODULES[viewId]) activateView(viewId);
  });
}

// ── Data refresh ────────────────────────────────────────────────────

async function refreshAll() {
  const t0 = performance.now();

  const [overview, mesh, tasks] = await Promise.allSettled([
    api.fetchOverview(),
    api.fetchMesh(),
    api.fetchTasksDistribution(),
  ]);

  if (overview.status === 'fulfilled' && !(overview.value instanceof Error)) {
    store.set('overview', overview.value);
  }
  if (mesh.status === 'fulfilled' && !(mesh.value instanceof Error)) {
    store.set('mesh', mesh.value);
  }
  if (tasks.status === 'fulfilled' && !(tasks.value instanceof Error)) {
    store.set('tasks', tasks.value);
  }

  const elapsed = Math.round(performance.now() - t0);
  store.set('lastRefresh', { ts: Date.now(), elapsed });

  const badge = document.getElementById('last-update');
  if (badge) badge.textContent = `Updated: ${new Date().toLocaleTimeString()}`;
}

// ── WebSocket handler ───────────────────────────────────────────────

function handleWsMessage(msg) {
  if (!msg || typeof msg !== 'object') return;

  if (msg.type === 'refresh') {
    refreshAll();
    return;
  }

  // Forward domain-specific events into the store so views can react
  if (msg.type && msg.data !== undefined) {
    store.set(`ws:${msg.type}`, msg.data);
  }
}

// ── Keyboard shortcuts ──────────────────────────────────────────────

function bindKeyboard() {
  document.addEventListener('keydown', (e) => {
    // Ctrl/Cmd + K opens command palette
    if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
      e.preventDefault();
      const palette = document.getElementById('cmd-palette');
      if (palette && typeof palette.open === 'function') palette.open();
    }
  });
}

// ── Theme persistence ────────────────────────────────────────────────

function bindThemePersistence() {
  // Listen for theme changes from <mn-theme-rotary> custom event
  document.addEventListener('mn-theme-change', (e) => {
    localStorage.setItem(
      'mn-theme',
      e.detail?.theme || document.documentElement.getAttribute('data-theme'),
    );
  });

  // Fallback: MutationObserver on data-theme in case rotary doesn't fire events
  const observer = new MutationObserver((mutations) => {
    for (const m of mutations) {
      if (m.attributeName === 'data-theme') {
        const theme = document.documentElement.getAttribute('data-theme');
        if (theme) localStorage.setItem('mn-theme', theme);
      }
    }
  });
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  });
}

// ── Init ────────────────────────────────────────────────────────────

async function init() {
  const registry = Maranello.ViewRegistry.getInstance();
  const nav = new Maranello.NavigationModel();
  orch = new Maranello.PanelOrchestrator(registry, nav);

  registerViews(registry);
  bindSidebarNav();
  bindCommandPalette();
  bindKeyboard();
  bindThemePersistence();

  // Mobile sidebar toggle
  if (typeof Maranello.initSidebarToggleAuto === 'function') {
    Maranello.initSidebarToggleAuto();
  }

  // Hash-based routing (browser back/forward)
  window.addEventListener('hashchange', handleHashChange);

  // Real-time updates
  const ws = connectDashboardWS(handleWsMessage);

  // Initial data load
  await refreshAll();

  // Open view from URL hash or fall back to default
  const initial = viewFromHash() || DEFAULT_VIEW;
  activateView(initial);

  // Periodic refresh
  const refreshTimer = setInterval(refreshAll, REFRESH_INTERVAL_MS);

  // Expose orchestrator for cross-view navigation (e.g. plan detail)
  window.__convergio = {
    orch,
    nav,
    registry,
    ws,
    refreshAll,
    openView: activateView,
    stopRefresh: () => clearInterval(refreshTimer),
  };
}

document.addEventListener('DOMContentLoaded', init);
