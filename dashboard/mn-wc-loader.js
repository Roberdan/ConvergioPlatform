;(function () {
  'use strict';

  const ns = (window.MaranelloEnhancer = window.MaranelloEnhancer || {});

  ns.WC_BASE = 'node_modules/maranello-luce-design-business/dist/wc/';
  ns.WC_MODULES = [
    'mn-toast', 'mn-modal', 'mn-tabs', 'mn-command-palette',
    'mn-system-status', 'mn-chart', 'mn-gauge', 'mn-speedometer',
    'mn-data-table', 'mn-detail-panel', 'mn-a11y',
    'mn-funnel', 'mn-hbar', 'mn-gantt', 'mn-okr',
    'mn-ferrari-control', 'mn-theme-toggle', 'mn-theme-rotary',
    'mn-profile', 'mn-mapbox',
  ];

  ns._wcLoaded = false;
  ns.M = () => window.Maranello;

  ns.loadWCs = async function loadWCs() {
    if (ns._wcLoaded) return true;
    try {
      const results = await Promise.allSettled(
        ns.WC_MODULES.map((m) => import(ns.WC_BASE + m + '.js')),
      );
      const ok = results.filter((r) => r.status === 'fulfilled').length;
      results.forEach((r, i) => {
        if (r.status === 'rejected') {
          console.warn(
            '[Maranello] WC ' + ns.WC_MODULES[i] + ':',
            r.reason?.message || 'load failed',
          );
        }
      });
      ns._wcLoaded = ok > 0;
      if (ns._wcLoaded) {
        console.log('[Maranello] ' + ok + '/' + ns.WC_MODULES.length + ' Web Components registered from CDN');
      }
      return ns._wcLoaded;
    } catch (e) {
      console.warn('[Maranello] WC load failed:', e.message);
      return false;
    }
  };
})();
