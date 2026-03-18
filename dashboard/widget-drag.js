/**
 * widget-drag.js — GridStack-based dashboard grid.
 * All drag, resize, and layout persistence via GridStack.
 */
(function () {
  const STORAGE_KEY = "dashGridLayoutV6";
  const GRID_ROWS = 10; // target rows to fill viewport

  function calcCellHeight() {
    const section = document.getElementById('dashboard-main-section');
    if (!section) return 70;
    const available = section.clientHeight;
    // Each cell = available / GRID_ROWS, minus margin (6px * 2)
    return Math.max(40, Math.floor((available - GRID_ROWS * 12) / GRID_ROWS));
  }

  function initGrid() {
    const el = document.getElementById('dashboard-grid');
    if (!el || !window.GridStack) return;

    const saved = loadLayout();
    const ch = calcCellHeight();
    const grid = GridStack.init({
      column: 12,
      cellHeight: ch,
      margin: 6,
      animate: false,
      float: true,
      columnOpts: { breakpoints: [{ w: 0, c: 12 }] },
      draggable: { handle: '.mn-widget__header' },
      resizable: { handles: 'se,e,s' },
    }, '#dashboard-grid');

    // Prevent GridStack from capturing clicks on interactive elements
    grid.el.addEventListener('mousedown', (e) => {
      if (e.target.closest('button, input, select, a, label, .mn-widget__action, .ops-tab, .plan-view-toggle, .analytics-range-btn, .activity-filter-btn, .kanban-card, .kanban-col')) {
        e.stopPropagation();
      }
    }, true);

    if (saved) {
      grid.load(saved, true);
    }

    grid.on('change', () => saveLayout(grid));

    // Trigger chart/brain resize when any widget resizes
    grid.on('resizestop', () => {
      window.dispatchEvent(new Event('resize'));
      const charts = window._charts;
      if (charts) {
        ['token', 'model', 'dist'].forEach(k => { if (charts[k]) charts[k].resize(); });
      }
      if (typeof window.brainResize === 'function') window.brainResize();
    });

    // Re-fit cellHeight on window resize
    let resizeTimer;
    window.addEventListener('resize', () => {
      clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        const newCH = calcCellHeight();
        grid.cellHeight(newCH);
      }, 200);
    });

    window._dashGrid = grid;
  }

  function saveLayout(grid) {
    try {
      const items = grid.save(false);
      localStorage.setItem(STORAGE_KEY, JSON.stringify(items));
    } catch {}
  }

  function loadLayout() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      return raw ? JSON.parse(raw) : null;
    } catch { return null; }
  }

  window.resetWidgetLayout = function () {
    localStorage.removeItem(STORAGE_KEY);
    localStorage.removeItem('dashActiveWidgetsV1');
    localStorage.removeItem('dashOnboardV1');
    ['dashGridLayoutV5', 'dashWidgetLayoutV4', 'dashWidgetLayoutV3', 'dashWidgetLayoutV2', 'dashWidgetLayout',
     'dashWidgetCollapsed', 'dashWidgetExpanded'].forEach(k => localStorage.removeItem(k));
    location.reload();
  };

  window.enableWidgetDrag = function () {};

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initGrid);
  } else {
    initGrid();
  }
})();
