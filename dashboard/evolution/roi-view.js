/**
 * roi-view.js — ROI summary view for the Evolution Engine dashboard.
 *
 * Renders weekly ROI KPIs using Maranello.kpiScorecard and mn- web components.
 * Data source: /data/evolution-roi-summary.json
 */

;(function () {
  'use strict';

  const ROI_DATA_URL = '/data/evolution-roi-summary.json';

  const KPI_CONFIG = [
    { key: 'experimentsRun', label: 'Experiments Run', unit: '' },
    { key: 'netROI', label: 'Net ROI', unit: 'pts' },
    { key: 'rollbacks', label: 'Rollbacks', unit: '' },
    { key: 'estimatedSavingsUsd', label: 'Est. Savings', unit: 'USD' },
  ];

  async function loadData() {
    const r = await fetch(ROI_DATA_URL, { cache: 'no-store' });
    if (!r.ok) throw new Error(`ROI load failed: ${r.status}`);
    return r.json();
  }

  function renderKpi(container, label, value, unit) {
    const formatted = typeof value === 'number'
      ? unit === 'USD' ? `$${value.toFixed(2)}` : value % 1 === 0 ? String(value) : value.toFixed(2)
      : String(value ?? '—');

    if (window.Maranello && typeof window.Maranello.kpiScorecard === 'function') {
      window.Maranello.kpiScorecard(container, { label, value: formatted, unit: unit !== 'USD' ? unit : '' });
      return;
    }

    // mn- web component fallback
    const el = document.createElement('mn-kpi');
    el.setAttribute('label', label);
    el.setAttribute('value', formatted);
    if (unit && unit !== 'USD') el.setAttribute('unit', unit);
    container.appendChild(el);
  }

  async function initRoiView(container) {
    if (!container) return;
    container.innerHTML = '<div class="roi-loading">Loading ROI…</div>';

    let summary;
    try { summary = await loadData(); }
    catch (err) {
      container.innerHTML = `<div class="roi-error">ROI unavailable: ${err.message}</div>`;
      return;
    }

    container.innerHTML = '';
    container.classList.add('roi-view');

    const header = document.createElement('div');
    header.className = 'roi-period';
    header.textContent = `Period: ${summary.period ?? '—'}`;
    container.appendChild(header);

    const grid = document.createElement('div');
    grid.className = 'roi-kpi-grid';
    container.appendChild(grid);

    KPI_CONFIG.forEach(({ key, label, unit }) => {
      const cell = document.createElement('div');
      cell.className = 'roi-kpi-cell';
      grid.appendChild(cell);
      renderKpi(cell, label, summary[key], unit);
    });
  }

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = { initRoiView };
  } else {
    window.EvolutionRoiView = { initRoiView };
  }
})();
