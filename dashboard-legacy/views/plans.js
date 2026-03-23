/**
 * Plans view — plan list table + detail side-panel with wave/task tree and Gantt.
 * Registered as a view factory in app.js; planDetail exported for side-panel.
 */

const { StateScaffold } = window.Maranello;

const TABLE_COLUMNS = JSON.stringify([
  { key: 'name', label: 'Plan' },
  { key: 'project_id', label: 'Project' },
  { key: 'status', label: 'Status' },
  { key: 'progress', label: 'Progress' },
  { key: 'execution_host', label: 'Host' },
  { key: 'created_at', label: 'Created' },
]);

/**
 * Format a plan row for the data table.
 * @param {object} p — raw plan from API
 * @returns {object} enriched row
 */
function planRow(p) {
  return {
    ...p,
    progress: `${p.tasks_done}/${p.tasks_total}`,
    status: window.statusDot(p.status) + ' ' + window.esc(p.status),
    created_at: p.created_at ? new Date(p.created_at).toLocaleDateString() : '',
  };
}

// ── Main view factory ─────────────────────────────────────────────────

/**
 * Plans list view.
 * @param {HTMLElement} container — mount target
 * @param {{api: object, store: object}} deps
 * @returns {Function} teardown callback
 */
export default function plans(container, { api, store }) {
  const scaffold = new StateScaffold(container, {
    state: 'loading',
    onRetry: () => refresh(),
  });

  const table = document.createElement('mn-data-table');
  table.setAttribute('columns', TABLE_COLUMNS);
  table.setAttribute('page-size', '15');
  table.setAttribute('selectable', '');
  container.appendChild(table);

  table.addEventListener('mn-row-click', (e) => {
    const plan = e.detail.row;
    if (window.__convergio?.orch) {
      window.__convergio.orch.open('plan-detail', 'side-panel', plan);
    }
  });

  async function refresh() {
    scaffold.state = 'loading';
    const result = await api.fetchPlanList();

    if (result instanceof Error || !result.plans) {
      scaffold.state = 'error';
      console.warn('[plans] fetchPlanList failed', result);
      return;
    }

    table.setData(result.plans.map(planRow));
    scaffold.state = 'ready';
  }

  refresh();
  const unsub = store.subscribe('plans', () => refresh());

  return () => {
    unsub();
    container.innerHTML = '';
  };
}

// ── Plan detail (side-panel) ──────────────────────────────────────────

/**
 * Status badge CSS class for Maranello tokens.
 * @param {string} status
 * @returns {string}
 */
function badgeClass(status) {
  const map = {
    done: 'mn-badge--success',
    in_progress: 'mn-badge--active',
    submitted: 'mn-badge--active',
    pending: 'mn-badge--warning',
    blocked: 'mn-badge--danger',
    cancelled: 'mn-badge--danger',
    skipped: 'mn-badge--muted',
    merging: 'mn-badge--active',
  };
  return map[status] || 'mn-badge--muted';
}

/**
 * Render a single task row inside a wave accordion.
 * @param {object} task
 * @returns {string} HTML
 */
function taskHtml(task) {
  const badge = `<span class="mn-badge ${badgeClass(task.status)}">${window.esc(task.status)}</span>`;
  const model = task.model
    ? `<span class="mn-text--secondary mn-text--sm">${window.esc(task.model)}</span>`
    : '';
  return `<div class="plan-detail__task">
    <span class="plan-detail__task-id">#${task.task_id}</span>
    <span class="plan-detail__task-title">${window.esc(task.title)}</span>
    ${badge} ${model}
  </div>`;
}

/**
 * Render wave accordion with its tasks.
 * @param {{wave: object, tasks: Array}} entry
 * @returns {string} HTML
 */
function waveHtml(entry) {
  const w = entry.wave;
  const pct = w.tasks_total > 0
    ? Math.round((w.tasks_done / w.tasks_total) * 100)
    : 0;
  const badge = `<span class="mn-badge ${badgeClass(w.status)}">${window.esc(w.status)}</span>`;
  const tasks = (entry.tasks || []).map(taskHtml).join('');

  return `<details class="plan-detail__wave" open>
    <summary class="plan-detail__wave-header">
      <strong>Wave ${w.wave_id}</strong> ${badge}
      <span class="mn-text--secondary">${w.tasks_done}/${w.tasks_total} (${pct}%)</span>
    </summary>
    <div class="plan-detail__wave-tasks">${tasks || '<em>No tasks</em>'}</div>
  </details>`;
}

/**
 * Build Gantt task items from the execution tree.
 * @param {Array} tree — wave/task tree from API
 * @returns {Array<{id, name, start, end, progress, group}>}
 */
function buildGanttTasks(tree) {
  const items = [];
  for (const entry of tree) {
    const w = entry.wave;
    for (const t of entry.tasks || []) {
      items.push({
        id: `t-${t.task_id}`,
        name: t.title || `Task #${t.task_id}`,
        start: t.started_at || t.created_at || new Date().toISOString(),
        end: t.completed_at || new Date().toISOString(),
        progress: t.status === 'done' ? 100 : t.status === 'in_progress' ? 50 : 0,
        group: `Wave ${w.wave_id}`,
      });
    }
  }
  return items;
}

/**
 * Plan detail view — opens in side-panel via orchestrator.
 * @param {HTMLElement} container
 * @param {object} data — plan row from table click
 * @param {{api: object}} deps
 * @returns {Function} teardown callback
 */
export function planDetail(container, data, { api }) {
  const scaffold = new StateScaffold(container, {
    state: 'loading',
    onRetry: () => refresh(),
  });

  const wrapper = document.createElement('div');
  wrapper.className = 'plan-detail';
  container.appendChild(wrapper);

  const tabs = document.createElement('mn-tabs');
  wrapper.appendChild(tabs);

  const treeTab = document.createElement('mn-tab');
  treeTab.setAttribute('label', 'Waves');
  tabs.appendChild(treeTab);

  const ganttTab = document.createElement('mn-tab');
  ganttTab.setAttribute('label', 'Timeline');
  tabs.appendChild(ganttTab);

  const ganttAvailable = !!customElements.get('mn-gantt');
  let ganttEl;
  if (ganttAvailable) {
    ganttEl = document.createElement('mn-gantt');
    ganttTab.appendChild(ganttEl);
  } else {
    console.warn('[plan-detail] mn-gantt not registered — showing placeholder');
    ganttEl = document.createElement('mn-placeholder');
    ganttEl.setAttribute('label', 'Gantt timeline — awaiting MLD delivery');
    ganttTab.appendChild(ganttEl);
  }

  async function refresh() {
    scaffold.state = 'loading';
    const result = await api.fetchPlanTree(data.id);

    if (result instanceof Error || !result.plan) {
      scaffold.state = 'error';
      console.warn('[plan-detail] fetchPlanTree failed', result);
      return;
    }

    const plan = result.plan;
    const tree = result.tree || [];

    // Header with plan summary
    const summaryHtml = plan.human_summary
      ? `<p class="plan-detail__summary">${window.esc(plan.human_summary)}</p>`
      : '';
    const header = `<div class="plan-detail__header">
      <h3>${window.esc(plan.name)}</h3>
      <span class="mn-badge ${badgeClass(plan.status)}">${window.esc(plan.status)}</span>
      ${summaryHtml}
    </div>`;

    // Wave tree
    treeTab.innerHTML = header + tree.map(waveHtml).join('');

    // Gantt timeline — only populate if mn-gantt is available
    if (ganttAvailable) {
      const ganttItems = buildGanttTasks(tree);
      if (ganttItems.length > 0) {
        ganttEl.setAttribute('tasks', JSON.stringify(ganttItems));
      } else {
        ganttTab.innerHTML = '<p class="mn-text--secondary">No task timeline data available.</p>';
        ganttTab.appendChild(ganttEl);
      }
    }

    scaffold.state = 'ready';
  }

  refresh();

  return () => {
    container.innerHTML = '';
  };
}
