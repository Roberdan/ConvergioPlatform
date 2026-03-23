/**
 * Approvals view — active plans awaiting human approval actions.
 * Shows plans with status='doing', supports Approve / Cancel / Pause.
 */
import { apiFetch } from '../lib/api-core.js';

const STYLE_ID = 'mn-approvals-view-style';
function injectStyles() {
  if (document.getElementById(STYLE_ID)) return;
  const s = document.createElement('style');
  s.id = STYLE_ID;
  s.textContent = `
    .apr-loading{color:var(--mn-text-muted);padding:1rem}
    .apr-error{color:var(--signal-danger,#ef4444);padding:.5rem}
    .apr-empty{color:var(--mn-text-muted);padding:1rem}
    .apr-actions{display:flex;gap:.5rem;flex-wrap:wrap}
    .apr-btn{font-size:.75rem;padding:.3rem .6rem;border-radius:4px;border:none;cursor:pointer;font-weight:500}
    .apr-btn-approve{background:var(--signal-success,#22c55e);color:#fff}
    .apr-btn-cancel{background:var(--signal-danger,#ef4444);color:#fff}
    .apr-btn-pause{background:var(--signal-warning,#f59e0b);color:#fff}`;
  document.head.appendChild(s);
}

function esc(s) {
  const d = document.createElement('div');
  d.textContent = String(s ?? '');
  return d.innerHTML;
}

function showLoading(el) {
  el.innerHTML = '<div class="apr-loading">Loading active plans...</div>';
}
function showError(el, msg) {
  el.innerHTML = `<div class="apr-error">${esc(msg)}</div>`;
}
function showEmpty(el) {
  el.innerHTML = '<div class="apr-empty">No active plans requiring approval.</div>';
}

function mkTable(rows) {
  const cols = [
    { key: 'id', label: 'ID' },
    { key: 'name', label: 'Name' },
    { key: 'wave', label: 'Wave' },
    { key: 'progress', label: 'Progress' },
    { key: 'actions', label: 'Actions' },
  ];
  const t = document.createElement('mn-data-table');
  t.setAttribute('columns', JSON.stringify(cols));
  // Render rows without actions column — actions rendered separately below
  const displayRows = rows.map((r) => ({
    id: r.id,
    name: esc(r.name),
    wave: r.current_wave ?? '-',
    progress: `${r.tasks_done ?? 0}/${r.tasks_total ?? 0}`,
    actions: '',
  }));
  t.setAttribute('rows', JSON.stringify(displayRows));
  return t;
}

async function handleApprove(planId, container) {
  const result = await apiFetch(`/api/plan-db/start/${planId}`, { method: 'POST' });
  if (result instanceof Error) {
    console.warn('[approvals] approve failed', planId, result);
    return;
  }
  await renderContent(container);
}

async function handleCancel(planId, container) {
  const reason = prompt('Reason for cancellation:');
  if (reason === null) return;
  const result = await apiFetch(`/api/plan-db/cancel/${planId}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ reason }),
  });
  if (result instanceof Error) {
    console.warn('[approvals] cancel failed', planId, result);
    return;
  }
  await renderContent(container);
}

async function handlePause(planId, container) {
  const result = await apiFetch('/api/coordinator/emit', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ event_type: 'pause_run', payload: { plan_id: planId } }),
  });
  if (result instanceof Error) {
    console.warn('[approvals] pause failed', planId, result);
    return;
  }
  await renderContent(container);
}

function mkActionButtons(row, container) {
  const wrap = document.createElement('div');
  wrap.className = 'apr-actions';
  wrap.dataset.planId = row.id;

  const approveBtn = document.createElement('button');
  approveBtn.className = 'apr-btn apr-btn-approve';
  approveBtn.textContent = 'Approve';
  approveBtn.setAttribute('aria-label', `Approve plan ${row.id}`);
  approveBtn.addEventListener('click', () => handleApprove(row.id, container));

  const cancelBtn = document.createElement('button');
  cancelBtn.className = 'apr-btn apr-btn-cancel';
  cancelBtn.textContent = 'Cancel';
  cancelBtn.setAttribute('aria-label', `Cancel plan ${row.id}`);
  cancelBtn.addEventListener('click', () => handleCancel(row.id, container));

  const pauseBtn = document.createElement('button');
  pauseBtn.className = 'apr-btn apr-btn-pause';
  pauseBtn.textContent = 'Pause';
  pauseBtn.setAttribute('aria-label', `Pause plan ${row.id}`);
  pauseBtn.addEventListener('click', () => handlePause(row.id, container));

  wrap.append(approveBtn, cancelBtn, pauseBtn);
  return wrap;
}

async function renderContent(container) {
  showLoading(container);

  const result = await apiFetch('/api/plan-db/list');
  if (result instanceof Error) {
    showError(container, 'Failed to load plans');
    console.warn('[approvals] fetch plans failed', result);
    return;
  }

  const all = Array.isArray(result) ? result : (result?.plans ?? []);
  const active = all.filter((p) => p.status === 'doing');

  if (active.length === 0) {
    showEmpty(container);
    return;
  }

  container.innerHTML = '';
  container.appendChild(mkTable(active));

  const actionsSection = document.createElement('div');
  actionsSection.style.cssText = 'margin-top:1rem;display:flex;flex-direction:column;gap:.75rem';
  for (const plan of active) {
    const row = document.createElement('div');
    row.style.cssText = 'display:flex;align-items:center;gap:1rem';
    const label = document.createElement('span');
    label.style.cssText = 'font-size:.8rem;color:var(--mn-text-muted);min-width:6rem';
    label.textContent = `#${plan.id}`;
    row.append(label, mkActionButtons(plan, container));
    actionsSection.appendChild(row);
  }
  container.appendChild(actionsSection);
}

/** @param {HTMLElement} container @param {{api: object, store: object}} deps */
export default function approvals(container, { api, store }) {
  injectStyles();
  container.innerHTML = '';
  renderContent(container);
  return () => { container.innerHTML = ''; };
}
