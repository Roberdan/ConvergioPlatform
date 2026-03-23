/**
 * Agents view — merged IPC coordination into unified agent management.
 * Tabs: Agents, Budget, Models, Skills, Locks.
 * Data sourced from lib/api-ipc.js endpoints.
 */
import {
  fetchIpcAgents, fetchIpcBudget, fetchIpcModels,
  fetchIpcSkills, fetchIpcLocks, fetchIpcConflicts, fetchIpcRouteHistory,
} from '../lib/api-ipc.js';

const STYLE_ID = 'mn-agents-view-style';
function injectStyles() {
  if (document.getElementById(STYLE_ID)) return;
  const s = document.createElement('style');
  s.id = STYLE_ID;
  s.textContent = `
    .agents-gauges{display:grid;grid-template-columns:repeat(auto-fill,minmax(200px,1fr));gap:1rem;padding:1rem 0}
    .agents-gauge{text-align:center}
    .agents-gauge .mn-text--sm{color:var(--mn-text-muted)}
    .agents-conflicts{padding:1rem 0}
    .agents-conflict{padding:.5rem;margin-bottom:.5rem;border-left:3px solid var(--signal-danger);background:var(--mn-surface-raised);border-radius:4px}
    .agents-loading{color:var(--mn-text-muted);padding:1rem}
    .agents-error{color:var(--signal-danger);padding:.5rem}
    .agents-empty{color:var(--mn-text-muted);padding:1rem}`;
  document.head.appendChild(s);
}

function esc(s) {
  const d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}
function showLoading(el, what) {
  el.innerHTML = `<div class="agents-loading">Loading ${what}...</div>`;
}
function showError(el, msg) {
  el.innerHTML = `<div class="agents-error">${esc(msg)}</div>`;
}
function showEmpty(el, msg) {
  el.innerHTML = `<div class="agents-empty">${msg}</div>`;
}

function mkTable(columns, rows) {
  const t = document.createElement('mn-data-table');
  t.setAttribute('columns', JSON.stringify(columns));
  t.setAttribute('rows', JSON.stringify(rows));
  return t;
}

async function renderSimpleTab(tab, label, fetchFn, columns, listKey) {
  showLoading(tab, label);
  const result = await fetchFn();
  if (result instanceof Error || (listKey === 'agents' && !result?.ok)) {
    showError(tab, `Failed to load ${label}`);
    console.warn(`[agents] fetch ${label} failed`, result);
    return;
  }
  const rows = result?.[listKey] || [];
  if (rows.length === 0) { showEmpty(tab, `No ${label} registered.`); return; }
  tab.innerHTML = '';
  tab.appendChild(mkTable(columns, rows));
}

const AGENT_COLS = [
  { key: 'agent_id', label: 'Agent ID' },
  { key: 'host', label: 'Host' },
  { key: 'status', label: 'Status' },
  { key: 'current_task', label: 'Current Task' },
  { key: 'last_heartbeat', label: 'Last Heartbeat' },
];
const MODEL_COLS = [
  { key: 'id', label: 'ID' },
  { key: 'name', label: 'Name' },
  { key: 'provider', label: 'Provider' },
  { key: 'context_window', label: 'Context Window' },
  { key: 'cost_per_token', label: 'Cost/Token' },
];
const SKILL_COLS = [
  { key: 'name', label: 'Name' },
  { key: 'agent', label: 'Agent' },
  { key: 'version', label: 'Version' },
  { key: 'enabled', label: 'Enabled' },
];
const ROUTE_COLS = [
  { key: 'timestamp', label: 'Time' },
  { key: 'model', label: 'Model' },
  { key: 'provider', label: 'Provider' },
  { key: 'tokens', label: 'Tokens' },
  { key: 'cost_usd', label: 'Cost ($)' },
];
const LOCK_COLS = [
  { key: 'file_pattern', label: 'File Pattern' },
  { key: 'agent', label: 'Agent' },
  { key: 'host', label: 'Host' },
  { key: 'locked_at', label: 'Locked At' },
];

async function renderBudgetTab(tab) {
  showLoading(tab, 'budget');
  const [budgetRes, routeRes] = await Promise.allSettled([
    fetchIpcBudget(),
    fetchIpcRouteHistory(),
  ]);
  tab.innerHTML = '';

  if (budgetRes.status === 'fulfilled' && !(budgetRes.value instanceof Error)) {
    const budgets = budgetRes.value?.budgets || [];
    if (budgets.length > 0) {
      const grid = document.createElement('div');
      grid.className = 'agents-gauges';
      for (const b of budgets) {
        const cell = document.createElement('div');
        cell.className = 'agents-gauge';
        const gauge = document.createElement('mn-gauge');
        gauge.setAttribute('value', String(b.budget_usd || 0));
        gauge.setAttribute('max', String(b.budget_usd || 100));
        gauge.setAttribute('label', b.subscription || b.provider || 'Budget');
        const info = document.createElement('div');
        info.className = 'mn-text--sm';
        info.textContent = `${b.provider} / ${b.plan} — ${b.status}`;
        if (b.alert) {
          info.style.color = 'var(--signal-warning)';
          info.textContent += ` [${b.alert}]`;
        }
        cell.append(gauge, info);
        grid.appendChild(cell);
      }
      tab.appendChild(grid);
    }
  } else {
    showError(tab, 'Failed to load budget data');
  }

  if (routeRes.status === 'fulfilled' && !(routeRes.value instanceof Error)) {
    const routes = routeRes.value?.routes || routeRes.value || [];
    if (Array.isArray(routes) && routes.length > 0) {
      const h = document.createElement('h4');
      h.textContent = 'Route History';
      h.style.margin = '1.5rem 0 0.5rem';
      tab.append(h, mkTable(ROUTE_COLS, routes));
    }
  }
}

async function renderLocksTab(tab) {
  showLoading(tab, 'locks');
  const [lockRes, conflictRes] = await Promise.allSettled([
    fetchIpcLocks(),
    fetchIpcConflicts(),
  ]);
  tab.innerHTML = '';

  if (lockRes.status === 'fulfilled' && !(lockRes.value instanceof Error)) {
    const locks = lockRes.value?.locks || [];
    if (locks.length > 0) tab.appendChild(mkTable(LOCK_COLS, locks));
    else showEmpty(tab, 'No active locks.');
  } else {
    showError(tab, 'Failed to load locks');
  }

  if (conflictRes.status === 'fulfilled' && !(conflictRes.value instanceof Error)) {
    const conflicts = conflictRes.value?.conflicts || [];
    if (conflicts.length > 0) {
      const sec = document.createElement('div');
      sec.className = 'agents-conflicts';
      const h = document.createElement('h4');
      h.textContent = 'Conflict Warnings';
      h.style.color = 'var(--signal-danger)';
      sec.appendChild(h);
      for (const c of conflicts) {
        const item = document.createElement('div');
        item.className = 'agents-conflict';
        item.textContent = c.message || c.description || JSON.stringify(c);
        sec.appendChild(item);
      }
      tab.appendChild(sec);
    }
  }
}

function mkTab(tabs, label) {
  const t = document.createElement('mn-tab');
  t.setAttribute('label', label);
  tabs.appendChild(t);
  return t;
}

/** @param {HTMLElement} container @param {{api: object, store: object}} deps */
export default function agents(container, { api, store }) {
  injectStyles();
  container.innerHTML = '';
  const tabs = document.createElement('mn-tabs');

  renderSimpleTab(mkTab(tabs, 'Agents'), 'agents', fetchIpcAgents, AGENT_COLS, 'agents');
  renderBudgetTab(mkTab(tabs, 'Budget'));
  renderSimpleTab(mkTab(tabs, 'Models'), 'models', fetchIpcModels, MODEL_COLS, 'models');
  renderSimpleTab(mkTab(tabs, 'Skills'), 'skills', fetchIpcSkills, SKILL_COLS, 'skills');
  renderLocksTab(mkTab(tabs, 'Locks'));

  container.appendChild(tabs);
  return () => { container.innerHTML = ''; };
}
