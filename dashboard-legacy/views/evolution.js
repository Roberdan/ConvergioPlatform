/**
 * Evolution view — proposals, experiments, and ROI.
 * Tabs: Proposals (mn-data-table + approve/reject), Experiments (before/after),
 * ROI (chart with success rate and rollbacks).
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
    .evo-actions{display:flex;gap:.5rem;margin-top:.25rem}
    .evo-actions button{font-size:.75rem;padding:.25rem .5rem;border-radius:4px;border:none;cursor:pointer}
    .evo-btn-approve{background:var(--signal-success,#22c55e);color:#fff}
    .evo-btn-reject{background:var(--signal-danger,#ef4444);color:#fff}
    .evo-metrics-pair{display:grid;grid-template-columns:1fr 1fr;gap:1rem;padding:.5rem 0}
    .evo-metrics-pair h4{margin:0 0 .25rem;font-size:.75rem;color:var(--mn-text-muted)}
    .evo-roi-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(160px,1fr));gap:1rem;padding:1rem 0}
    .evo-stat{padding:1rem;background:var(--mn-surface-raised);border-radius:8px;text-align:center}
    .evo-stat__value{font-size:1.5rem;font-weight:700;color:var(--mn-accent)}
    .evo-stat__label{font-size:.75rem;color:var(--mn-text-muted);margin-top:.25rem}
    .evo-audit{font-size:.75rem;color:var(--mn-text-muted);margin-top:.5rem}`;
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
  { key: 'target_metric', label: 'Target Metric' },
  { key: 'expected_delta', label: 'Expected Delta' },
  { key: 'blast_radius', label: 'Blast Radius' },
  { key: 'status', label: 'Status' },
];

const EXPERIMENT_COLS = [
  { key: 'proposal_id', label: 'Proposal' },
  { key: 'hypothesis', label: 'Hypothesis' },
  { key: 'mode', label: 'Mode' },
  { key: 'result', label: 'Result' },
  { key: 'started_at', label: 'Started' },
  { key: 'completed_at', label: 'Completed' },
];

async function handleAction(id, action, tab) {
  const reason = prompt(`Reason for ${action}:`);
  if (reason === null) return;
  const result = await apiFetch(`/api/evolution/proposals/${id}/${action}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ reason, actor: 'dashboard-user' }),
  });
  if (result instanceof Error) {
    console.warn(`[evolution] ${action} failed`, result);
    return;
  }
  await renderProposalsTab(tab);
}

function addActionButtons(container, rows, tab) {
  const pending = rows.filter((r) => r.status === 'pending');
  if (pending.length === 0) return;
  for (const row of pending) {
    const div = document.createElement('div');
    div.className = 'evo-actions';
    div.dataset.proposalId = row.id;
    const approveBtn = document.createElement('button');
    approveBtn.className = 'evo-btn-approve';
    approveBtn.textContent = 'Approve';
    approveBtn.addEventListener('click', () => handleAction(row.id, 'approve', tab));
    const rejectBtn = document.createElement('button');
    rejectBtn.className = 'evo-btn-reject';
    rejectBtn.textContent = 'Reject';
    rejectBtn.addEventListener('click', () => handleAction(row.id, 'reject', tab));
    div.append(approveBtn, rejectBtn);
    container.appendChild(div);
  }
}

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
  addActionButtons(tab, rows, tab);
}

function renderMetricsPair(container, before, after) {
  const wrap = document.createElement('div');
  wrap.className = 'evo-metrics-pair';
  const bDiv = document.createElement('div');
  const bHead = document.createElement('h4');
  bHead.textContent = 'Before';
  bDiv.appendChild(bHead);
  bDiv.appendChild(document.createTextNode(before || 'N/A'));
  const aDiv = document.createElement('div');
  const aHead = document.createElement('h4');
  aHead.textContent = 'After';
  aDiv.appendChild(aHead);
  aDiv.appendChild(document.createTextNode(after || 'N/A'));
  wrap.append(bDiv, aDiv);
  container.appendChild(wrap);
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
  for (const exp of rows) {
    if (exp.before_metrics || exp.after_metrics) {
      renderMetricsPair(tab, exp.before_metrics, exp.after_metrics);
    }
  }
}

function renderRoiStat(grid, label, value) {
  const cell = document.createElement('div');
  cell.className = 'evo-stat';
  const val = document.createElement('div');
  val.className = 'evo-stat__value';
  val.textContent = String(value ?? 0);
  const lbl = document.createElement('div');
  lbl.className = 'evo-stat__label';
  lbl.textContent = label;
  cell.append(val, lbl);
  grid.appendChild(cell);
}

async function renderRoiTab(tab) {
  showLoading(tab, 'ROI');
  const result = await apiFetch('/api/evolution/roi');
  if (result instanceof Error) {
    showError(tab, 'Failed to load ROI data');
    console.warn('[evolution] fetch ROI failed', result);
    return;
  }
  tab.innerHTML = '';
  const grid = document.createElement('div');
  grid.className = 'evo-roi-grid';
  renderRoiStat(grid, 'Experiments Run', result.experimentsRun);
  renderRoiStat(grid, 'Successes', result.successes);
  renderRoiStat(grid, 'Rollbacks', result.rollbacks);
  renderRoiStat(grid, 'Success Rate', `${result.successRate ?? 0}%`);
  tab.appendChild(grid);
  if (Array.isArray(result.proposalsByStatus) && result.proposalsByStatus.length > 0) {
    const chart = document.createElement('mn-chart');
    chart.setAttribute('type', 'bar');
    chart.setAttribute('data', JSON.stringify(result.proposalsByStatus));
    chart.setAttribute('x-key', 'status');
    chart.setAttribute('y-key', 'count');
    chart.setAttribute('aria-label', 'Proposals by status');
    tab.appendChild(chart);
  }
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
  renderRoiTab(mkTab(tabs, 'ROI'));
  container.appendChild(tabs);
  return () => { container.innerHTML = ''; };
}
