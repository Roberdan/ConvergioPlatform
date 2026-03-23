/**
 * roi-widget.js — Weekly ROI summary widget for the Evolution Engine dashboard.
 *
 * Renders 4 KPIs using Maranello.kpiScorecard:
 *   - Experiments run
 *   - Net delta score
 *   - Rollbacks
 *   - Estimated savings (USD)
 *
 * Data source: /data/evolution-roi-summary.json (written by RoiTracker)
 */

;(function () {
  'use strict';

  const ROI_DATA_URL = '/data/evolution-roi-summary.json';

  const KPI_CONFIG = [
    { key: 'experimentsRun', label: 'Experiments Run', unit: '', icon: 'flask' },
    { key: 'netDeltaScore', label: 'Net Δ Score', unit: 'pts', icon: 'trending-up' },
    { key: 'rollbacks', label: 'Rollbacks', unit: '', icon: 'undo' },
    { key: 'estimatedSavingsUsd', label: 'Est. Savings', unit: 'USD', icon: 'dollar-sign' },
  ];

  /**
   * Loads the ROI summary JSON from the data endpoint.
   * @returns {Promise<Object>} The ROI summary object.
   */
  async function loadRoiData() {
    const response = await fetch(ROI_DATA_URL, { cache: 'no-store' });
    if (!response.ok) {
      throw new Error(`ROI data fetch failed: ${response.status}`);
    }
    return response.json();
  }

  /**
   * Renders a single KPI card into the container.
   * Uses Maranello.kpiScorecard if available; falls back to plain HTML.
   *
   * @param {HTMLElement} container
   * @param {string} label
   * @param {string|number} value
   * @param {string} unit
   */
  function renderKpi(container, label, value, unit) {
    const formatted =
      typeof value === 'number'
        ? unit === 'USD'
          ? `$${value.toFixed(2)}`
          : value % 1 === 0
            ? String(value)
            : value.toFixed(2)
        : String(value ?? '—');

    if (window.Maranello && typeof window.Maranello.kpiScorecard === 'function') {
      window.Maranello.kpiScorecard(container, {
        label,
        value: formatted,
        unit: unit !== 'USD' ? unit : '',
      });
      return;
    }

    // Fallback renderer
    const card = document.createElement('div');
    card.className = 'roi-kpi-card';
    card.innerHTML = `
      <span class="roi-kpi-label">${label}</span>
      <span class="roi-kpi-value">${formatted}${unit && unit !== 'USD' ? ' ' + unit : ''}</span>
    `;
    container.appendChild(card);
  }

  /**
   * Initialises the ROI widget inside the given container element.
   *
   * @param {HTMLElement} container
   */
  async function initRoiWidget(container) {
    if (!container) return;

    container.innerHTML = '<div class="roi-loading" aria-busy="true">Loading ROI data…</div>';

    let summary;
    try {
      summary = await loadRoiData();
    } catch (err) {
      container.innerHTML = `<div class="roi-error">ROI data unavailable: ${err.message}</div>`;
      return;
    }

    container.innerHTML = '';
    container.classList.add('roi-widget');

    const period = document.createElement('div');
    period.className = 'roi-period';
    period.textContent = `Period: ${summary.period ?? '—'}`;
    container.appendChild(period);

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

  // Export for ES module consumers and window global
  if (typeof module !== 'undefined' && module.exports) {
    module.exports = { initRoiWidget };
  } else {
    window.EvolutionRoiWidget = { initRoiWidget };
  }
})();
