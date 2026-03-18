/**
 * Maranello Enhancer v6 — thin orchestrator
 * Pinned DS CDN release: @v4.5.0
 */
;(function () {
  'use strict';

  const ns = (window.MaranelloEnhancer = window.MaranelloEnhancer || {});

  ns._active = false;
  ns._orig = ns._orig || {};
  ns._added = ns._added || [];
  ns._cmdActions = ns._cmdActions || {};

  const TOAST_MAP = { error: 'danger', warn: 'warning' };

  ns.isActive = () => ns._active;
  ns.addEl = (el) => { ns._added.push(el); return el; };
  ns.bindData = (key, cb) => {
    if (window.Maranello && typeof window.Maranello.bind === 'function') {
      try { window.Maranello.bind(key, cb); } catch (_) {}
    }
  };
  ns.emitData = (key, value) => {
    if (window.Maranello && typeof window.Maranello.emit === 'function') {
      try { window.Maranello.emit(key, value); } catch (_) {}
    }
  };
  ns.bindChart = (target, options) => {
    if (window.Maranello && typeof window.Maranello.bindChart === 'function') {
      try { return window.Maranello.bindChart(target, options || {}); } catch (_) {}
    }
    return null;
  };
  ns.whenVisible = (el, init) => {
    if (!el || typeof init !== 'function') return;
    let started = false;
    const run = () => {
      if (started) return;
      if (el.offsetWidth > 0 && el.offsetHeight > 0) {
        started = true;
        init();
      }
    };
    run();
    if (started) return;
    if (typeof IntersectionObserver === 'function') {
      const io = new IntersectionObserver((entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          run();
          if (started) io.disconnect();
        }
      }, { threshold: 0.1 });
      io.observe(el);
    }
    if (typeof ResizeObserver === 'function') {
      const ro = new ResizeObserver(() => {
        run();
        if (started) ro.disconnect();
      });
      ro.observe(el);
    }
    setTimeout(run, 250);
  };
  // brain IntersectionObserver handoff helper for neural viz deferred init.
  ns.observeBrainVisibility = (brainEl, init) => ns.whenVisible(brainEl, init);

  function captureOriginals() {
    ['showToast', 'renderKpi', 'renderTaskPipeline', 'renderTokenChart', 'openPlanSidebar'].forEach((fn) => {
      if (!ns._orig[fn] && typeof window[fn] === 'function') ns._orig[fn] = window[fn];
    });
    const mgr = window.chatTabs;
    if (mgr && mgr._render && !ns._orig.chatTabsRender) ns._orig.chatTabsRender = mgr._render.bind(mgr);
  }

  function restoreOriginals() {
    Object.keys(ns._orig).forEach((fn) => {
      if (fn === 'chatTabsRender') {
        const mgr = window.chatTabs;
        if (mgr) mgr._render = ns._orig.chatTabsRender;
        return;
      }
      window[fn] = ns._orig[fn];
    });
  }

  function enhanceToasts() {
    window.showToast = function showToastMaranello(title, msg, link, type) {
      if (arguments.length <= 2 && ['info', 'success', 'warning', 'error', 'warn', 'danger'].includes(msg)) {
        type = msg;
        msg = '';
      }
      const t = document.createElement('mn-toast');
      t.setAttribute('title', title || '');
      t.setAttribute('message', msg || '');
      t.setAttribute('type', TOAST_MAP[type] || type || 'info');
      t.setAttribute('duration', '8000');
      if (link && typeof link === 'string') {
        t.addEventListener('click', () => { location.hash = link.replace(/.*#/, '#'); });
        t.style.cursor = 'pointer';
      }
      document.body.appendChild(t);
    };
  }

  function enhanceModals() {
    window.maranelloModal = function maranelloModal(title, bodyHTML, opts) {
      const m = document.createElement('mn-modal');
      m.setAttribute('title', title || '');
      m.innerHTML = bodyHTML || '';
      document.body.appendChild(m);
      requestAnimationFrame(() => m.open());
      if (opts && opts.onClose) m.addEventListener('mn-close', opts.onClose, { once: true });
      return m;
    };
  }

  function enhanceCommandPalette() {
    if (document.querySelector('mn-command-palette')) return;
    const cp = document.createElement('mn-command-palette');
    const items = [
      { id: 'theme', text: 'Switch Theme', shortcut: 'T', group: 'Navigation' },
      { id: 'refresh', text: 'Refresh Dashboard', shortcut: 'R', group: 'Actions' },
      { id: 'tasks', text: 'Go to Tasks', group: 'Navigation' },
      { id: 'admin', text: 'Go to Admin', group: 'Navigation' },
      { id: 'chat', text: 'Go to Chat', group: 'Navigation' },
      { id: 'brain', text: 'Go to Brain', group: 'Navigation' },
      { id: 'opt', text: 'Run Optimize', group: 'Actions' },
    ];
    ns._cmdActions.theme = () => document.getElementById('theme-toggle')?.click();
    ns._cmdActions.refresh = () => window.refreshAll?.();
    ns._cmdActions.tasks = () => document.querySelector('[data-section="dashboard-main-section"]')?.click();
    ns._cmdActions.admin = () => document.querySelector('[data-section="dashboard-admin-section"]')?.click();
    ns._cmdActions.chat = () => document.querySelector('[data-section="dashboard-chat-section"]')?.click();
    ns._cmdActions.brain = () => document.querySelector('[data-section="dashboard-brain-section"]')?.click();
    ns._cmdActions.opt = () => document.querySelector('.header-optimize-btn')?.click();

    cp.setAttribute('items', JSON.stringify(items));
    cp.setAttribute('placeholder', 'Search commands…');
    cp.addEventListener('mn-select', (e) => {
      const item = e.detail?.item;
      if (item?.id && ns._cmdActions[item.id]) ns._cmdActions[item.id]();
    });
    document.body.appendChild(ns.addEl(cp));
  }

  function enhanceSystemStatus() {
    // mn-system-status is now in the HTML header directly — just verify it exists
    const existing = document.querySelector('mn-system-status');
    if (existing) return;
    const ss = document.createElement('mn-system-status');
    ss.setAttribute('poll-interval', '15000');
    ss.setAttribute('services', JSON.stringify([
      { name: 'API', url: '/api/health' },
      { name: 'Mesh Traffic', url: '/api/mesh/traffic' },
    ]));
    const hr = document.querySelector('.header-right');
    if (hr) hr.insertBefore(ns.addEl(ss), hr.firstChild);
  }

  function enhanceA11y() {
    // mn-a11y is now in the HTML directly — no injection needed
  }

  function enhanceTabs() {
    const mgr = window.chatTabs;
    if (!mgr || !mgr._render) return;
    mgr._render = function renderTabsEnhanced() {
      if (!ns._active) return ns._orig.chatTabsRender?.();
      const root = document.getElementById('chat-tabs-root');
      if (!root) return;
      const mt = document.createElement('mn-tabs');
      mt.setAttribute('active', String(this.tabs.findIndex((t) => t.id === this.activeId)));
      this.tabs.forEach((tab) => {
        const p = document.createElement('mn-tab');
        p.setAttribute('label', tab.title || 'Chat');
        const content = document.createElement('div');
        content.id = 'chat-tab-content-' + String(tab.id || '').replace(/[^a-zA-Z0-9_-]/g, '_');
        p.appendChild(content);
        mt.appendChild(p);
      });
      root.innerHTML = '';
      root.appendChild(mt);
      mt.addEventListener('mn-tab-change', (e) => {
        const idx = e.detail?.index ?? 0;
        if (this.tabs[idx]) this.switchTo(this.tabs[idx].id);
      });
    };
  }

  function enhanceDataTables() {
    // Convergio 2.0: task-pipeline.js renders directly with wave grouping.
    // The mn-data-table WC flattens the hierarchy and loses status dots.
    return;
  }

  ns._autoResizeCleanups = [];
  ns._sidebarCleanup = null;

  ns.enableResponsive = function enableResponsive() {
    const api = ns.M?.();
    if (!api) return;

    // autoResize: wrap all visible chart canvases
    if (typeof api.autoResizeAll === 'function') {
      try { ns._autoResizeCleanups.push(api.autoResizeAll()); } catch (_) {}
    } else if (typeof api.autoResize === 'function') {
      document.querySelectorAll('canvas[data-chart]').forEach((c) => {
        try { ns._autoResizeCleanups.push(api.autoResize(c)); } catch (_) {}
      });
    }

    // Mobile sidebar toggle
    if (typeof api.initSidebarToggleAuto === 'function') {
      try { ns._sidebarCleanup = api.initSidebarToggleAuto(); } catch (_) {}
    }
  };

  ns.disableResponsive = function disableResponsive() {
    ns._autoResizeCleanups.forEach((fn) => { try { fn?.(); } catch (_) {} });
    ns._autoResizeCleanups = [];
    try { ns._sidebarCleanup?.(); } catch (_) {}
    ns._sidebarCleanup = null;
  };

  function enhanceDetailPanel() {
    if (typeof window.openPlanSidebar !== 'function') return;
    window.openPlanSidebar = async function openPlanSidebarEnhanced(planId) {
      if (!ns._active) return ns._orig.openPlanSidebar?.(planId);
      const data = await fetch('/api/plan/' + planId).then((r) => r.json()).catch(() => null);
      if (!data) return ns._orig.openPlanSidebar?.(planId);
      let panel = document.querySelector('mn-detail-panel');
      if (!panel) {
        panel = document.createElement('mn-detail-panel');
        document.body.appendChild(ns.addEl(panel));
      }
      const plan = data.plan || data;
      const sections = [{ label: 'Overview', fields: [{ label: 'Name', value: plan.name || planId }, { label: 'Status', value: plan.status || 'unknown' }, { label: 'Progress', value: (plan.tasks_done || 0) + '/' + (plan.tasks_total || 0) }, { label: 'Host', value: plan.execution_host || '—' }] }];
      if (data.waves) {
        sections.push({ label: 'Waves', fields: data.waves.map((w) => ({ label: w.wave_id + ' ' + (w.name || ''), value: w.status + ' (' + w.tasks_done + '/' + w.tasks_total + ')' })) });
      }
      panel.setAttribute('title', plan.name || 'Plan ' + planId);
      panel.setAttribute('sections', JSON.stringify(sections));
      if (typeof panel.open === 'function') panel.open();
      else panel.removeAttribute('hidden');
    };
  }

  function triggerRerender() { window.refreshAll?.(); }

  async function activate() {
    if (ns._active) return;
    if (!ns.M?.()) return console.warn('[Maranello] CDN IIFE not loaded — window.Maranello unavailable');
    try {
      await ns.loadWCs?.();
      captureOriginals();
      enhanceToasts();
      enhanceModals();
      enhanceCommandPalette();
      enhanceSystemStatus();
      enhanceA11y();
      enhanceTabs();
      enhanceDataTables();
      ns.enhanceCharts?.();
      ns.enhanceKpiGauges?.();
      enhanceDetailPanel();
      setTimeout(() => { ns.enhanceBrainViz?.(); }, 1500);
      ns.applyChartColorsNow?.();
      ns.enableResponsive?.();
      ns._active = true;
      triggerRerender();
      console.log('[Maranello] CDN enhancements active (v' + (ns.M()?.VERSION || '?') + ')');
    } catch (e) {
      console.error('[Maranello] Activation failed, rolling back:', e);
      deactivate();
    }
  }

  function deactivate() {
    // Maranello is always the foundation — deactivate is a no-op.
    // Legacy: kept for API compatibility but does nothing.
    console.log('[Maranello] deactivate() called but Maranello is always-on');
  }

  window.maranelloEnhancer = { activate, deactivate, isActive: ns.isActive };
})();
