/**
 * widget-catalog.js — Widget Catalog System for Convergio Dashboard
 * 
 * Manages available widgets, add/remove from GridStack grid,
 * catalog drawer UI, and persistence of active widget set.
 */
(function () {
  'use strict';

  const ACTIVE_KEY = 'dashActiveWidgetsV1';
  const TEMPLATES_ID = 'widget-templates';

  // Widget definitions — order = default catalog display order
  const CATALOG = [
    { id: 'ops-zone-widget',      title: 'Ops Zone',        icon: '⚙️', desc: 'Active missions, recent plans, history log', defaultPos: { x: 0, y: 0, w: 3, h: 5 }, minW: 2, minH: 3 },
    { id: 'mesh-panel',           title: 'Mesh Hub',        icon: '🔗', desc: 'Peer network status, sync controls',         defaultPos: { x: 3, y: 0, w: 5, h: 2 }, minW: 3, minH: 2 },
    { id: 'brain-widget',         title: 'Augmented Brain', icon: '🧠', desc: 'Neural network visualization',              defaultPos: { x: 3, y: 2, w: 5, h: 3 }, minW: 3, minH: 2 },
    { id: 'activity-feed-widget', title: 'Activity',        icon: '📡', desc: 'GitHub events and system activity feed',     defaultPos: { x: 8, y: 0, w: 4, h: 3 }, minW: 2, minH: 2 },
    { id: 'mesh-latency-widget',  title: 'Mesh Latency',    icon: '⏱️', desc: 'Real-time network latency gauges',          defaultPos: { x: 8, y: 3, w: 4, h: 2 }, minW: 2, minH: 2 },
    { id: 'task-pipeline-widget', title: 'Task Pipeline',   icon: '📋', desc: 'All plan tasks with status and progress',   defaultPos: { x: 0, y: 5, w: 4, h: 5 }, minW: 3, minH: 2 },
    { id: 'plan-status-widget',   title: 'Plan Status',     icon: '📊', desc: 'Timeline and Kanban views of plans',        defaultPos: { x: 4, y: 5, w: 4, h: 5 }, minW: 3, minH: 2 },
    { id: 'analytics-triptych',   title: 'Analytics',       icon: '📈', desc: 'Token burn, cost by model, task dist',      defaultPos: { x: 8, y: 5, w: 4, h: 5 }, minW: 2, minH: 2 },
    { id: 'ipc-budget-widget',    title: 'Budget Intelligence', icon: '💰', desc: 'Subscription gauge with 70/85/95% thresholds', defaultPos: { x: 0, y: 10, w: 3, h: 4 }, minW: 2, minH: 2 },
    { id: 'ipc-router-widget',    title: 'Task Router',     icon: '🔀', desc: 'Task→model routing history and patterns',   defaultPos: { x: 3, y: 10, w: 3, h: 4 }, minW: 2, minH: 2 },
    { id: 'ipc-skills-widget',    title: 'Skills Matrix',   icon: '🧩', desc: 'Agent×skills heatmap matrix',              defaultPos: { x: 6, y: 10, w: 3, h: 4 }, minW: 2, minH: 2 },
    { id: 'ipc-models-widget',    title: 'Model Registry',  icon: '🤖', desc: 'Model registry + auth status indicators',  defaultPos: { x: 9, y: 10, w: 3, h: 4 }, minW: 2, minH: 2 },
  ];

  // Default active: core 8 widgets (IPC widgets start in catalog)
  const DEFAULT_ACTIVE = new Set([
    'ops-zone-widget', 'mesh-panel', 'brain-widget', 'activity-feed-widget',
    'mesh-latency-widget', 'task-pipeline-widget', 'plan-status-widget', 'analytics-triptych'
  ]);

  // --- State ---
  let activeWidgetIds = null;

  function loadActiveSet() {
    try {
      const raw = localStorage.getItem(ACTIVE_KEY);
      if (raw) activeWidgetIds = new Set(JSON.parse(raw));
      else activeWidgetIds = new Set(DEFAULT_ACTIVE);
    } catch { activeWidgetIds = new Set(DEFAULT_ACTIVE); }
  }

  function saveActiveSet() {
    if (!activeWidgetIds) return;
    localStorage.setItem(ACTIVE_KEY, JSON.stringify([...activeWidgetIds]));
  }

  function isActive(id) {
    return !activeWidgetIds || activeWidgetIds.has(id);
  }

  // --- Template Storage ---
  function getTemplateContainer() {
    let c = document.getElementById(TEMPLATES_ID);
    if (!c) {
      c = document.createElement('div');
      c.id = TEMPLATES_ID;
      c.hidden = true;
      document.body.appendChild(c);
    }
    return c;
  }

  // --- Remove Widget ---
  function removeWidget(widgetId) {
    const grid = window._dashGrid;
    if (!grid) return;
    const gsItem = grid.el.querySelector(`.grid-stack-item[gs-id="${widgetId}"]`);
    if (!gsItem) return;

    // Move content to template storage before GridStack destroys the DOM
    const content = gsItem.querySelector('.grid-stack-item-content');
    if (content) {
      const stored = document.createElement('div');
      stored.id = 'tpl-' + widgetId;
      stored.innerHTML = content.innerHTML;
      getTemplateContainer().appendChild(stored);
    }

    grid.removeWidget(gsItem, true, false);

    if (!activeWidgetIds) {
      activeWidgetIds = new Set(CATALOG.map(c => c.id));
    }
    activeWidgetIds.delete(widgetId);
    saveActiveSet();

    updateCatalogBadge();
    if (typeof window._catalogDrawerRefresh === 'function') window._catalogDrawerRefresh();
  }

  // --- Add Widget ---
  function addWidget(widgetId) {
    const grid = window._dashGrid;
    if (!grid) return;
    const def = CATALOG.find(c => c.id === widgetId);
    if (!def) return;

    // Check if already on grid
    if (grid.el.querySelector(`.grid-stack-item[gs-id="${widgetId}"]`)) return;

    // Recover stored content or use the original from HTML
    let innerHtml = '';
    const stored = document.getElementById('tpl-' + widgetId);
    if (stored) {
      innerHtml = stored.innerHTML;
      stored.remove();
    }

    // Build grid item — set gs-id explicitly for querySelector lookups
    const itemEl = document.createElement('div');
    itemEl.className = 'grid-stack-item';
    itemEl.setAttribute('gs-id', widgetId);
    itemEl.setAttribute('gs-w', def.defaultPos.w);
    itemEl.setAttribute('gs-h', def.defaultPos.h);
    itemEl.setAttribute('gs-min-w', def.minW);
    itemEl.setAttribute('gs-min-h', def.minH);
    const contentEl = document.createElement('div');
    contentEl.className = 'grid-stack-item-content';
    contentEl.innerHTML = innerHtml;
    itemEl.appendChild(contentEl);

    // GridStack v12: append to grid DOM, then makeWidget()
    grid.el.appendChild(itemEl);
    grid.makeWidget(itemEl);

    // Scroll to the newly added widget so user sees it
    setTimeout(() => {
      const section = document.getElementById('dashboard-main-section');
      const widgetRect = itemEl.getBoundingClientRect();
      const sectionRect = section?.getBoundingClientRect();
      if (section && sectionRect && widgetRect.bottom > sectionRect.bottom) {
        section.scrollTo({ top: section.scrollTop + widgetRect.top - sectionRect.top - 20, behavior: 'smooth' });
      }
      // Flash the widget to draw attention
      const w = itemEl.querySelector('.mn-widget');
      if (w) {
        w.style.boxShadow = '0 0 20px var(--accent, #FFC72C)';
        w.style.transition = 'box-shadow 1.5s ease-out';
        setTimeout(() => { w.style.boxShadow = ''; }, 1500);
      }
    }, 200);

    if (!activeWidgetIds) {
      activeWidgetIds = new Set(CATALOG.map(c => c.id));
    }
    activeWidgetIds.add(widgetId);
    saveActiveSet();

    // Re-init traffic lights on new widget
    if (typeof window._initTrafficLightsOn === 'function') {
      const widget = itemEl.querySelector('.mn-widget');
      if (widget) window._initTrafficLightsOn(widget);
    }

    // Trigger refresh to populate data
    setTimeout(() => {
      if (typeof refreshAll === 'function') refreshAll();
      if (typeof window.brainResize === 'function') window.brainResize();
    }, 300);

    updateCatalogBadge();
    if (typeof window._catalogDrawerRefresh === 'function') window._catalogDrawerRefresh();

    // Close drawer after add so user sees the widget appear
    const drawer = document.getElementById('widget-catalog-drawer');
    if (drawer && !drawer.hidden) drawer.hidden = true;
  }

  // --- Initial Setup: Remove inactive widgets from grid ---
  function applyActiveSet() {
    loadActiveSet();
    if (!activeWidgetIds) return; // null = all active (first load)
    const grid = window._dashGrid;
    if (!grid) return;

    const toRemove = [];
    CATALOG.forEach(def => {
      if (!activeWidgetIds.has(def.id)) {
        const gsItem = grid.el.querySelector(`.grid-stack-item[gs-id="${def.id}"]`);
        if (gsItem) toRemove.push({ id: def.id, item: gsItem });
      }
    });

    // Batch remove after collecting (avoid mutation during iteration)
    grid.batchUpdate();
    toRemove.forEach(({ id, item }) => {
      const content = item.querySelector('.grid-stack-item-content');
      if (content) {
        const stored = document.createElement('div');
        stored.id = 'tpl-' + id;
        stored.innerHTML = content.innerHTML;
        getTemplateContainer().appendChild(stored);
      }
      grid.removeWidget(item, true, false);
    });
    grid.batchUpdate(false);
  }

  // --- Catalog Drawer ---
  function buildCatalogDrawer() {
    let drawer = document.getElementById('widget-catalog-drawer');
    if (drawer) return;

    drawer = document.createElement('div');
    drawer.id = 'widget-catalog-drawer';
    drawer.className = 'catalog-drawer';
    drawer.hidden = true;

    const header = document.createElement('div');
    header.className = 'catalog-drawer__header';
    header.innerHTML = '<span class="catalog-drawer__title">Widget Catalog</span>' +
      '<button class="catalog-drawer__close mn-btn mn-btn--ghost" onclick="toggleWidgetCatalog()">✕</button>';
    drawer.appendChild(header);

    const body = document.createElement('div');
    body.className = 'catalog-drawer__body';
    body.id = 'catalog-drawer-body';
    drawer.appendChild(body);

    document.body.appendChild(drawer);
    refreshCatalogDrawer();
  }

  function refreshCatalogDrawer() {
    const body = document.getElementById('catalog-drawer-body');
    if (!body) return;

    const grid = window._dashGrid;
    const onGrid = new Set();
    if (grid) {
      grid.el.querySelectorAll('.grid-stack-item[gs-id]').forEach(el => {
        onGrid.add(el.getAttribute('gs-id'));
      });
    }

    body.innerHTML = CATALOG.map(def => {
      const active = onGrid.has(def.id);
      return `<div class="catalog-card ${active ? 'catalog-card--active' : ''}" data-widget-id="${def.id}">
        <div class="catalog-card__icon">${def.icon}</div>
        <div class="catalog-card__info">
          <div class="catalog-card__title">${def.title}</div>
          <div class="catalog-card__desc">${def.desc}</div>
        </div>
        <button class="catalog-card__btn mn-btn mn-btn--sm ${active ? 'mn-btn--ghost' : 'mn-btn--accent'}"
          onclick="${active ? `removeWidgetById('${def.id}')` : `addWidgetById('${def.id}')`}">
          ${active ? 'Remove' : 'Add'}
        </button>
      </div>`;
    }).join('');
  }
  window._catalogDrawerRefresh = refreshCatalogDrawer;

  function updateCatalogBadge() {
    const badge = document.getElementById('catalog-badge');
    if (!badge) return;
    const totalDefs = CATALOG.length;
    const onGrid = activeWidgetIds ? activeWidgetIds.size : totalDefs;
    const inactive = totalDefs - onGrid;
    badge.textContent = inactive;
    badge.style.display = inactive > 0 ? '' : 'none';
  }

  // --- Global API ---
  window.removeWidgetById = removeWidget;
  window.addWidgetById = addWidget;
  window.toggleWidgetCatalog = function () {
    const drawer = document.getElementById('widget-catalog-drawer');
    if (!drawer) { buildCatalogDrawer(); toggleWidgetCatalog(); return; }
    drawer.hidden = !drawer.hidden;
    if (!drawer.hidden) refreshCatalogDrawer();
  };
  window.WIDGET_CATALOG = CATALOG;

  // Init after GridStack is ready
  function init() {
    if (!window._dashGrid) {
      setTimeout(init, 200);
      return;
    }
    applyActiveSet();
    buildCatalogDrawer();
    updateCatalogBadge();
  }
  setTimeout(init, 400);
})();
