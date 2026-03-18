// convergio-layout.js — Convergio 2.1 layout interactions (GridStack edition)

(function() {

  // --- macOS Traffic Light Buttons ---
  function initTrafficLightsOn(w) {
    if (!w.id) return;
    const header = w.querySelector('.mn-widget__header');
    if (!header || header.querySelector('.traffic-lights')) return;
    const gsItem = w.closest('.grid-stack-item');

    const tl = document.createElement('div');
    tl.className = 'traffic-lights';

    // RED = remove widget from dashboard
    const red = document.createElement('button');
    red.className = 'tl-dot tl-red';
    red.title = 'Remove from dashboard';
    red.addEventListener('click', (e) => {
      e.stopPropagation();
      if (typeof window.removeWidgetById === 'function') {
        window.removeWidgetById(gsItem?.getAttribute('gs-id') || w.id);
      }
    });

    // YELLOW = minimize (collapse to header)
    const yellow = document.createElement('button');
    yellow.className = 'tl-dot tl-yellow';
    yellow.title = 'Minimize';
    yellow.addEventListener('click', (e) => {
      e.stopPropagation();
      w.classList.toggle('collapsed');
      if (window._dashGrid && gsItem) {
        if (w.classList.contains('collapsed')) {
          gsItem._origH = parseInt(gsItem.getAttribute('gs-h')) || 4;
          window._dashGrid.update(gsItem, { h: 1 });
        } else if (gsItem._origH) {
          window._dashGrid.update(gsItem, { h: gsItem._origH });
        }
      }
    });

    // GREEN = maximize (fill viewport below header)
    const green = document.createElement('button');
    green.className = 'tl-dot tl-green';
    green.title = 'Maximize';
    green.addEventListener('click', (e) => {
      e.stopPropagation();
      w.classList.remove('collapsed');
      if (window._dashGrid && gsItem) {
        if (!gsItem._isMaximized) {
          gsItem._origPos = {
            x: parseInt(gsItem.getAttribute('gs-x')),
            y: parseInt(gsItem.getAttribute('gs-y')),
            w: parseInt(gsItem.getAttribute('gs-w')),
            h: parseInt(gsItem.getAttribute('gs-h'))
          };
          // Calculate rows to fill viewport using actual cellHeight
          const section = document.getElementById('dashboard-main-section');
          const availH = section ? section.clientHeight : window.innerHeight - 160;
          const g = window._dashGrid;
          const cellH = (typeof g.getCellHeight === 'function' ? g.getCellHeight() : 60) + 12;
          const rows = Math.max(6, Math.floor(availH / cellH));
          window._dashGrid.update(gsItem, { x: 0, y: 0, w: 12, h: rows });
          gsItem._isMaximized = true;
          // Scroll to top so the maximized widget is fully visible
          if (section) section.scrollTop = 0;
        } else if (gsItem._origPos) {
          window._dashGrid.update(gsItem, gsItem._origPos);
          gsItem._isMaximized = false;
        }
      }
      window.dispatchEvent(new Event('resize'));
    });

    tl.appendChild(red);
    tl.appendChild(yellow);
    tl.appendChild(green);
    header.insertBefore(tl, header.firstChild);
  }

  function initAllTrafficLights() {
    document.querySelectorAll('.grid-stack-item-content .mn-widget').forEach(initTrafficLightsOn);
  }

  // Expose for widget-catalog.js to call on newly added widgets
  window._initTrafficLightsOn = initTrafficLightsOn;

  setTimeout(initAllTrafficLights, 300);

  // --- Ops Zone Tabs ---
  document.addEventListener('click', (e) => {
    const tab = e.target.closest('.ops-tab');
    if (!tab) return;
    const tabName = tab.dataset.tab;
    document.querySelectorAll('.ops-tab').forEach(t => t.classList.remove('active'));
    tab.classList.add('active');
    document.querySelectorAll('.ops-tab-content').forEach(c => c.hidden = true);
    const target = document.getElementById('ops-tab-' + tabName);
    if (target) target.hidden = false;
  });

  // --- Analytics Range Tabs ---
  document.addEventListener('click', (e) => {
    const btn = e.target.closest('.analytics-range-btn');
    if (!btn) return;
    document.querySelectorAll('.analytics-range-btn').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
  });

  // --- Activity Feed Filter ---
  document.addEventListener('click', (e) => {
    const btn = e.target.closest('.activity-filter-btn');
    if (!btn) return;
    document.querySelectorAll('.activity-filter-btn').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    const filter = btn.dataset.filter;
    const items = document.querySelectorAll('#activity-feed-content .feed-item');
    items.forEach(item => {
      if (filter === 'all') {
        item.hidden = false;
        return;
      }
      item.hidden = !item.classList.contains('feed-' + filter);
    });
  });

  // --- Plan View Toggle (Timeline/Kanban) ---
  document.addEventListener('click', (e) => {
    const btn = e.target.closest('.plan-view-toggle');
    if (!btn) return;
    document.querySelectorAll('.plan-view-toggle').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    const view = btn.dataset.view;
    document.querySelectorAll('.plan-status-view').forEach(v => v.hidden = true);
    if (view === 'timeline') {
      const el = document.getElementById('plan-timeline-container');
      if (el) el.hidden = false;
    } else {
      const el = document.getElementById('kanban-board');
      if (el) el.hidden = false;
    }
  });

  // --- Brain Expand (Fullscreen Overlay) ---
  window.expandBrain = function() {
    const container = document.getElementById('brain-canvas-container');
    if (!container) return;

    const overlay = document.createElement('div');
    overlay.id = 'brain-immersive-overlay';
    overlay.tabIndex = 0;
    overlay.style.cssText = 'position:fixed;inset:0;z-index:1500;background:var(--bg-deep,#0a0a0a);cursor:zoom-out;animation:brainExpand 0.4s ease-out';

    const clone = container.cloneNode(false);
    clone.className = 'brain-immersive';
    clone.style.cssText = 'width:100%;height:100%;position:relative;overflow:hidden;';

    const close = () => {
      overlay.style.animation = 'brainCollapse 0.3s ease-in';
      setTimeout(() => overlay.remove(), 300);
      container.classList.remove('brain-hidden');
      if (typeof window.brainResize === 'function') window.brainResize();
    };

    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) close();
    });
    overlay.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') close();
    });

    const closeBtn = document.createElement('button');
    closeBtn.className = 'mn-btn mn-btn--ghost';
    closeBtn.style.cssText = 'position:absolute;top:16px;right:16px;z-index:1501;font-size:20px;color:var(--text)';
    closeBtn.textContent = '✕';
    closeBtn.onclick = close;
    overlay.appendChild(closeBtn);

    const controls = document.getElementById('brain-controls');
    if (controls) {
      const ctrlClone = controls.cloneNode(true);
      ctrlClone.style.cssText = 'position:absolute;bottom:16px;left:50%;transform:translateX(-50%);z-index:1501;display:flex;gap:8px;background:rgba(0,0,0,0.6);padding:8px 16px;border-radius:8px';
      overlay.appendChild(ctrlClone);
    }

    overlay.appendChild(clone);
    document.body.appendChild(overlay);
    overlay.focus();

    container.classList.add('brain-hidden');
    if (typeof window.brainExpandTo === 'function') window.brainExpandTo(clone);
  };
})();
