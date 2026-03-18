;(function () {
  'use strict';

  const ns = (window.MaranelloEnhancer = window.MaranelloEnhancer || {});

  ns.PALETTE = null; // resolved live via Maranello.palette()
  ns.STATUS_COLORS = null; // resolved live via Maranello.palette()

  ns.getPalette = function getPalette() {
    const api = ns.M?.();
    if (api?.palette) {
      const p = api.palette();
      return [p.accent, p.signalDanger, p.signalOk, p.arancio, p.info, '#c084fc', '#f97316'];
    }
    return ['#FFC72C', '#DC0000', '#00A651', '#D4622B', '#4ea8de', '#c084fc', '#f97316'];
  };

  ns.getStatusColors = function getStatusColors() {
    const api = ns.M?.();
    if (api?.palette) {
      const p = api.palette();
      return {
        done: p.signalOk, in_progress: p.accent, submitted: p.info,
        blocked: p.signalDanger, pending: '#888', cancelled: p.signalDanger, skipped: '#555',
      };
    }
    return {
      done: '#00A651', in_progress: '#FFC72C', submitted: '#4ea8de',
      blocked: '#DC0000', pending: '#888', cancelled: '#DC0000', skipped: '#555',
    };
  };

  ns.makeCanvas = function makeCanvas(w, h, stretch) {
    const c = document.createElement('canvas');
    c.width = w;
    c.height = h;
    c.style.cssText = stretch
      ? 'width:100%;height:' + h + 'px;display:block;'
      : 'width:' + w + 'px;height:' + h + 'px;display:block;margin:0 auto;';
    return c;
  };

  ns.enhanceCharts = function enhanceCharts() {
    const C = () => ns.M?.()?.charts;

    if (typeof window.renderTokenChart !== 'function') return;
    window.renderTokenChart = function renderTokenChartEnhanced(daily) {
      if (!ns._active || !C()) return ns._orig?.renderTokenChart?.(daily);
      const origCanvas = document.getElementById('token-chart');
      if (!origCanvas) return ns._orig?.renderTokenChart?.(daily);
      const container = origCanvas.parentElement;
      if (!container) return;
      origCanvas.style.display = 'none';
      container.querySelectorAll('.mn-token-enhanced').forEach((el) => el.remove());
      if (!Array.isArray(daily) || !daily.length) return;

      const recent = daily.slice(-14);
      const pal = ns.getPalette?.() || ['#FFC72C', '#DC0000', '#00A651'];
      const pDanger = pal[1] || '#DC0000';
      const pWarn = pal[0] || '#FFC72C';
      const pOk = pal[2] || '#00A651';
      const barData = recent.map((d) => {
        const total = ((d.input || 0) + (d.output || 0)) / 1e6;
        const cost = d.cost || 0;
        const color = cost > 500 ? pDanger : cost > 100 ? pWarn : pOk;
        return { label: (d.day || '').substring(5), value: Math.round(total * 10) / 10, color };
      });

      const wrapper = document.createElement('div');
      wrapper.className = 'mn-token-enhanced';
      const width = container.offsetWidth || 460;
      const barCanvas = ns.makeCanvas(width, 150, true);
      wrapper.appendChild(barCanvas);

      const sparkRow = document.createElement('div');
      sparkRow.style.cssText = 'display:flex;align-items:center;gap:12px;margin-top:8px;';
      const sparkLabel = document.createElement('span');
      sparkLabel.className = 'mn-micro';
      sparkLabel.style.cssText = 'color:var(--grigio-chiaro,#888);font-size:9px;letter-spacing:2px;text-transform:uppercase;white-space:nowrap;';
      sparkLabel.textContent = 'Cost $';
      sparkRow.appendChild(sparkLabel);
      const sparkCanvas = ns.makeCanvas(width - 60, 28, true);
      sparkRow.appendChild(sparkCanvas);

      const costToday = document.createElement('span');
      const lastCost = recent[recent.length - 1]?.cost || 0;
      costToday.className = 'mn-micro';
      costToday.style.cssText = 'color:' + (lastCost > 500 ? pDanger : lastCost > 100 ? pWarn : pOk) + ';font-size:11px;font-weight:700;white-space:nowrap;';
      costToday.textContent = '$' + Math.round(lastCost);
      sparkRow.appendChild(costToday);
      wrapper.appendChild(sparkRow);

      container.appendChild(ns.addEl ? ns.addEl(wrapper) : wrapper);
      C().barChart(barCanvas, barData, { width: barCanvas.width, height: 150, color: pWarn });
      C().sparkline(sparkCanvas, recent.map((d) => d.cost || 0), { color: pDanger, width: sparkCanvas.width, height: 28 });
      ns.M?.().chartInteract?.(barCanvas, { showTooltip: true });
    };
  };

  ns.applyChartColorsNow = function applyChartColorsNow() {
    const bound =
      ns.bindChart?.('#token-chart', { palette: 'auto' }) ||
      ns.bindChart?.('#model-chart', { palette: 'auto' }) ||
      ns.bindChart?.('#dist-chart', { palette: 'auto' });
    if (bound) return;
    const s = getComputedStyle(document.documentElement);
    const p = [];
    for (let i = 1; i <= 10; i++) {
      const c = s.getPropertyValue('--chart-' + i).trim();
      if (c) p.push(c);
    }
    if (p.length) {
      const borderBg = s.getPropertyValue('--mn-surface').trim() || '#111';
      const textColor = s.getPropertyValue('--mn-text').trim() || '#c8c8c8';
      const mutedColor = s.getPropertyValue('--mn-text-muted').trim() || '#888';
      ns._applyPalette(p, borderBg, textColor, mutedColor, 'rgba(255,199,44,0.06)');
    }
  };

  ns.resetChartColors = function resetChartColors() {
    const defaults = ['#00f0ff', '#ff00c8', '#ffd700', '#00ff88', '#ff4444', '#4ea8de', '#c084fc', '#f97316', '#06b6d4', '#a3e635'];
    const s = getComputedStyle(document.documentElement);
    const textColor = s.getPropertyValue('--text').trim() || '#aaa';
    const mutedColor = s.getPropertyValue('--text-dim').trim() || '#666';
    ns._applyPalette(defaults, 'transparent', textColor, mutedColor, 'rgba(255,255,255,0.06)');
  };

  ns._applyPalette = function applyPalette(pal, borderBg) {
    if (typeof Chart === 'undefined' || !Chart.instances) return;
    Object.values(Chart.instances).forEach((chart) => {
      try {
        if (!chart?.data?.datasets) return;
        chart.data.datasets.forEach((ds, i) => {
          const c = pal[i % pal.length];
          if (chart.config.type === 'doughnut' || chart.config.type === 'pie') {
            ds.backgroundColor = pal.slice(0, ds.data?.length || pal.length);
            ds.borderColor = borderBg;
            ds.borderWidth = 2;
            return;
          }
          ds.borderColor = c;
          ds.backgroundColor = c + '33';
          ds.pointBackgroundColor = c;
        });
        chart.update('none');
      } catch (e) {
        console.warn('[Maranello] Chart palette error:', e.message);
      }
    });
  };
})();
