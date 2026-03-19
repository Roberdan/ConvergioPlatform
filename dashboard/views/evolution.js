/**
 * Evolution view — displays optimization engine status.
 * Tabs: Proposals, Experiments, Metrics.
 * Data sourced from /api/evolution/* endpoints.
 */
import { apiFetch } from '../lib/api-core.js';

const STYLE_ID = 'mn-evolution-view-style';
function injectStyles() {
  if (document.getElementById(STYLE_ID)) return;
  const s = document.createElement('style');
  s.id = STYLE_ID;
  s.textContent = `
    .evo-loading{color:var(--mn-text-muted);padding:1rem}
    .evo-error{color:var(--signal-danger);padding:.5rem}
    .evo-empty{color:var(--mn-text-muted);padding:1rem}
    .evo-summary{display:grid;grid-template-columns:repeat(auto-fill,minmax(180px,1fr));gap:1rem;padding:1rem 0}
    .evo-stat{padding:1rem;background:var(--mn-surface-raised);border-radius:8px;text-align:center}
    .evo-stat__value{font-size:1.5rem;font-weight:700;color:var(--mn-accent)}
    .evo-stat__label{font-size:0.75rem;color:var(--mn-text-muted);margin-top:0.25rem}`;
  document.head.appendChild(s);
}

function esc(s) {
  const d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}

function showLoading(el, what) {
  el.innerHTML = `<div class="evo-loading">Loading ${what}...</div>`;
}
function showError(el, msg) {
  el.innerHTML = `<div class="evo-error">${esc(msg)}</div>`;
}
function showEmpty(el, msg) {
  el.innerHTML = `<div class="evo-empty">${msg}</div>`;
}

function mkTable(columns, rows) {
  const t = document.createElement('mn-data-table');
  t.setAttribute('columns', JSON.stringify(columns));
  t.setAttribute('rows', JSON.stringify(rows));
  return t;
}

const PROPOSAL_COLS = [
  { key: 'id', label: 'ID' },
  { key: 'hypothesis', label: 'Hypothesis' },
  { key: 'targetMetric', label: 'Target Metric' },
  { key: 'expectedDelta', label: 'Expected Delta' },
  { key: 'blastRadius', label: 'Blast Radius' },
  { key: 'status', label: 'Status' },
];

const EXPERIMENT_COLS = [
  { key: 'proposalId', label: 'Proposal' },
  { key: 'mode', label: 'Mode' },
  { key: 'result', label: 'Result' },
  { key: 'startedAt', label: 'Started' },
  { key: 'completedAt', label: 'Completed' },
];

const METRIC_COLS = [
  { key: 'name', label: 'Name' },
  { key: 'value', label: 'Value' },
  { key: 'family', label: 'Family' },
  { key: 'timestamp', label: 'Timestamp' },
];

async function renderProposalsTab(tab) {
  showLoading(tab, 'proposals');
  const result = await apiFetch('/api/evolution/proposals');
  if (result instanceof Error) {
    showError(tab, 'Failed to load proposals');
    console.warn('[evolution] fetch proposals failed', result);
    return;
  }
  const rows = result?.proposals || result || [];
  if (!Array.isArray(rows) || rows.length === 0) {
    showEmpty(tab, 'No active proposals.');
    return;
  }
  tab.innerHTML = '';
  tab.appendChild(mkTable(PROPOSAL_COLS, rows));
}

async function renderExperimentsTab(tab) {
  showLoading(tab, 'experiments');
  const result = await apiFetch('/api/evolution/experiments');
  if (result instanceof Error) {
    showError(tab, 'Failed to load experiments');
    console.warn('[evolution] fetch experiments failed', result);
    return;
  }
  const rows = result?.experiments || result || [];
  if (!Array.isArray(rows) || rows.length === 0) {
    showEmpty(tab, 'No experiments recorded.');
    return;
  }
  tab.innerHTML = '';
  tab.appendChild(mkTable(EXPERIMENT_COLS, rows));
}

async function renderMetricsTab(tab) {
  showLoading(tab, 'metrics');
  const result = await apiFetch('/api/evolution/metrics');
  if (result instanceof Error) {
    showError(tab, 'Failed to load metrics');
    console.warn('[evolution] fetch metrics failed', result);
    return;
  }
  const rows = result?.metrics || result || [];
  if (!Array.isArray(rows) || rows.length === 0) {
    showEmpty(tab, 'No metrics collected.');
    return;
  }
  tab.innerHTML = '';

  // Summary cards for latest values by family
  const families = new Map();
  for (const m of rows) {
    const fam = m.family || 'general';
    if (!families.has(fam)) families.set(fam, []);
    families.get(fam).push(m);
  }
  if (families.size > 0) {
    const grid = document.createElement('div');
    grid.className = 'evo-summary';
    for (const [fam, metrics] of families) {
      const cell = document.createElement('div');
      cell.className = 'evo-stat';
      const val = document.createElement('div');
      val.className = 'evo-stat__value';
      val.textContent = String(metrics.length);
      const label = document.createElement('div');
      label.className = 'evo-stat__label';
      label.textContent = fam;
      cell.append(val, label);
      grid.appendChild(cell);
    }
    tab.appendChild(grid);
  }

  tab.appendChild(mkTable(METRIC_COLS, rows));
}

function mkTab(tabs, label) {
  const t = document.createElement('mn-tab');
  t.setAttribute('label', label);
  tabs.appendChild(t);
  return t;
}

/** @param {HTMLElement} container @param {{api: object, store: object}} deps */
export default function evolution(container, { api, store }) {
  injectStyles();
  container.innerHTML = '';
  const tabs = document.createElement('mn-tabs');

  renderProposalsTab(mkTab(tabs, 'Proposals'));
  renderExperimentsTab(mkTab(tabs, 'Experiments'));
  renderMetricsTab(mkTab(tabs, 'Metrics'));

  container.appendChild(tabs);
  return () => { container.innerHTML = ''; };
}
