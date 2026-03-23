// app.js — Convergio Control Room orchestrator
// Wires Maranello runtime, API clients, reactive store, and view modules.
// 3-zone layout: command strip + main + brain strip.
import * as api from './lib/api-core.js';
import * as store from './lib/store.js';
import { connectDashboardWS } from './lib/ws.js';
import { initBrainStrip } from './lib/brain-strip.js';
import { initDrawerChat } from './widgets/drawer-chat.js';
import { createDrawer } from './widgets/drawer-bottom.js';
import { getQueryParams, applyEmbeddedMode, isEmbedded } from './lib/embed.js';

const REFRESH_INTERVAL_MS = 16000;
const DEFAULT_VIEW = 'overview';
// Supported query params: ?mode=embedded (adds .mode-embedded), ?tab=, ?brain_mode=embedded

// View factories — lazy-loaded via dynamic import
const VIEW_MODULES = {
  overview: () => import('./views/overview.js'),
  plans: () => import('./views/plans.js'),
  mesh: () => import('./views/mesh.js'),
  brain: () => import('./views/brain.js'),
  agents: () => import('./views/agents.js'),
  evolution: () => import('./views/evolution.js'),
  approvals: () => import('./views/approvals.js'),
  admin: () => import('./views/admin.js'),
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
    link.classList.toggle(
      'mn-sidebar__link--active',
      link.dataset.view === viewId
    );
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

function bindCommandPalette(drawerToggle) {
  const palette = document.getElementById('cmd-palette');
  if (!palette) return;
  const navItems = Object.keys(VIEW_MODULES).map((id) => ({ text: viewTitle(id), group: 'Navigation' }));
  const cmdItems = [{ text: 'Toggle Terminal', group: 'Panels', shortcut: 'Ctrl+`' }];
  palette.items = JSON.stringify([...navItems, ...cmdItems]);
  palette.addEventListener('mn-select', (e) => {
    const label = e.detail.item.text;
    if (label === 'Toggle Terminal' && drawerToggle) { drawerToggle(); return; }
    const viewId = label.toLowerCase();
    if (VIEW_MODULES[viewId]) activateView(viewId);
  });
}

// ── Data refresh ────────────────────────────────────────────────────

async function refreshAll() {
  const t0 = performance.now();
  const [overview, mesh, tasks] = await Promise.allSettled([
    api.fetchOverview(), api.fetchMesh(), api.fetchTasksDistribution(),
  ]);
  const ok = (r) => r.status === 'fulfilled' && !(r.value instanceof Error);
  if (ok(overview)) store.set('overview', overview.value);
  if (ok(mesh)) store.set('mesh', mesh.value);
  if (ok(tasks)) store.set('tasks', tasks.value);
  store.set('lastRefresh', { ts: Date.now(), elapsed: Math.round(performance.now() - t0) });
  const badge = document.getElementById('last-update');
  if (badge) badge.textContent = `Updated: ${new Date().toLocaleTimeString()}`;
}

// ── WebSocket handler ───────────────────────────────────────────────

function handleWsMessage(msg) {
  if (!msg || typeof msg !== 'object') return;
  if (msg.type === 'refresh') { refreshAll(); return; }
  if (msg.type && msg.data !== undefined) store.set(`ws:${msg.type}`, msg.data);
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
  const persist = () => {
    const t = document.documentElement.getAttribute('data-theme');
    if (t) localStorage.setItem('mn-theme', t);
  };
  document.addEventListener('mn-theme-change', persist);
  new MutationObserver(persist).observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  });
}

// ── Init ────────────────────────────────────────────────────────────

async function init() {
  const qp = getQueryParams();

  // Apply embedded mode before any DOM wiring — hides sidebar/command strip
  if (qp.mode === 'embedded') applyEmbeddedMode();

  // Expose brain_mode param so brain/canvas.js can read it
  if (qp.brainMode === 'embedded') {
    window.__convergioBrainModeForced = 'embedded';
  }

  const registry = Maranello.ViewRegistry.getInstance();
  const nav = new Maranello.NavigationModel();
  orch = new Maranello.PanelOrchestrator(registry, nav);

  registerViews(registry);
  bindSidebarNav();

  // Bottom terminal drawer (Ctrl+` to toggle) — skip in embedded mode
  const termDrawer = isEmbedded() ? null : createDrawer();

  if (termDrawer) bindCommandPalette(termDrawer.toggle);
  bindKeyboard();
  bindThemePersistence();
  if (!isEmbedded()) initBrainStrip();
  initDrawerChat();

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

  // ?tab= param overrides hash, then hash, then default
  const tabParam = qp.tab;
  const initial = (tabParam && VIEW_MODULES[tabParam])
    ? tabParam
    : viewFromHash() || DEFAULT_VIEW;
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
