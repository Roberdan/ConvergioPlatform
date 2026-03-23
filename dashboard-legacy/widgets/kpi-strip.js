/**
 * KPI Strip widget factory for DashboardRenderer.
 * Renders a horizontal row of KPI stat cards with optional trend indicators.
 * Widget type: 'kpi-strip'
 */

const STYLE_ID = 'mn-kpi-strip-style';

function injectStyles() {
  if (document.getElementById(STYLE_ID)) return;
  const style = document.createElement('style');
  style.id = STYLE_ID;
  style.textContent = `
    .mn-kpi-strip {
      display: flex;
      gap: 1rem;
      flex-wrap: wrap;
      padding: 0.5rem 0;
    }
    .mn-kpi-strip .mn-card--stat {
      display: flex;
      flex-direction: column;
      align-items: center;
      min-width: 120px;
      flex: 1 1 0;
      padding: 1rem;
      border-radius: var(--mn-radius, 8px);
      background: var(--mn-surface);
      border: 1px solid var(--mn-border);
    }
    .mn-kpi-strip .mn-card__label {
      font-size: 0.75rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--mn-text-muted);
      margin-bottom: 0.25rem;
    }
    .mn-kpi-strip .mn-card__value {
      font-size: 1.5rem;
      font-weight: 700;
      color: var(--mn-text);
    }
    .mn-kpi-strip .mn-card__trend {
      font-size: 0.75rem;
      margin-top: 0.25rem;
    }
    .mn-kpi-strip .mn-card__trend--up {
      color: var(--signal-ok);
    }
    .mn-kpi-strip .mn-card__trend--down {
      color: var(--signal-danger);
    }
    .mn-kpi-strip .mn-card__trend--flat {
      color: var(--signal-warning);
    }
  `;
  document.head.appendChild(style);
}

function esc(s) {
  const d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}

function trendHtml(item) {
  if (item.prev == null || item.value == null) return '';
  const current = Number(item.value);
  const prev = Number(item.prev);
  if (Number.isNaN(current) || Number.isNaN(prev)) return '';

  const diff = current - prev;
  if (diff === 0) {
    return '<span class="mn-card__trend mn-card__trend--flat">~ 0%</span>';
  }

  const pct = prev !== 0 ? Math.round((diff / Math.abs(prev)) * 100) : 0;
  const arrow = diff > 0 ? '\u2191' : '\u2193';
  const cls = diff > 0 ? 'up' : 'down';
  return `<span class="mn-card__trend mn-card__trend--${cls}">${arrow} ${Math.abs(pct)}%</span>`;
}

function buildCard(item) {
  return `
    <div class="mn-card mn-card--stat">
      <span class="mn-card__label">${esc(item.label)}</span>
      <span class="mn-card__value">${esc(String(item.value))}</span>
      ${trendHtml(item)}
    </div>`;
}

/**
 * Render a KPI strip into the given container.
 * @param {HTMLElement} container - Target element
 * @param {Array<{label: string, value: string|number, prev?: number}>} data - KPI items
 */
export function renderKpiStrip(container, data) {
  injectStyles();
  container.className = 'mn-kpi-strip';

  if (!Array.isArray(data) || data.length === 0) {
    container.innerHTML = '';
    return;
  }

  container.innerHTML = data.map(buildCard).join('');
}

export default renderKpiStrip;
