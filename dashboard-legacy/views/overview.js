/**
 * Overview view — main dashboard landing page.
 * Registered as a view factory in app.js.
 * Uses Maranello.DashboardRenderer for widget grid layout.
 * Includes nightly jobs widget (last 5 via mn-data-table), mn-gauge for mesh health,
 * and system stats in KPI strip.
 */

const { DashboardRenderer, StateScaffold } = window.Maranello;

const NIGHTLY_COLS = [
  { key: 'job_name', label: 'Job' },
  { key: 'status', label: 'Status' },
  { key: 'duration_sec', label: 'Duration (s)' },
  { key: 'started_at', label: 'Started' },
];

const SCHEMA = {
  rows: [
    { columns: [{ type: 'kpi-strip', dataKey: 'kpis', span: 12 }] },
    {
      columns: [
        { type: 'chart', dataKey: 'tokenBurn', span: 6, options: { chartType: 'sparkline' } },
        { type: 'chart', dataKey: 'modelCost', span: 6, options: { chartType: 'donut' } },
      ],
    },
    {
      columns: [
        { type: 'chart', dataKey: 'taskDist', span: 4, options: { chartType: 'barChart' } },
        { type: 'gauge', dataKey: 'meshHealth', span: 4, options: { label: 'Mesh Health', unit: '%' } },
        { type: 'table', dataKey: 'activity', span: 4 },
      ],
    },
    {
      columns: [
        { type: 'table', dataKey: 'nightlyJobs', span: 12, options: { label: 'Nightly Jobs' } },
      ],
    },
  ],
};

/**
 * Format large numbers into human-readable shorthand.
 * @param {number} n
 * @returns {string}
 */
function formatNumber(n) {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K';
  return String(n);
}

/**
 * Build KPI strip items from overview response, including system stats.
 * @param {object} ov — overview API payload
 * @returns {Array<{label: string, value: string|number}>}
 */
function buildKpis(ov) {
  const kpis = [
    { label: 'Active Plans', value: ov.plans_active },
    { label: 'Agents Running', value: ov.agents_running },
    { label: 'Today Tokens', value: formatNumber(ov.today_tokens) },
    { label: 'Today Cost', value: '$' + ov.today_cost.toFixed(2) },
    { label: 'Mesh Online', value: `${ov.mesh_online}/${ov.mesh_total}` },
    { label: 'Lines Changed', value: formatNumber(ov.today_lines_changed) },
  ];
  if (ov.cpu_pct !== undefined) {
    kpis.push({ label: 'CPU', value: ov.cpu_pct + '%' });
  }
  if (ov.mem_used_mb !== undefined && ov.mem_total_mb !== undefined) {
    kpis.push({ label: 'Memory', value: `${formatNumber(ov.mem_used_mb)}/${formatNumber(ov.mem_total_mb)} MB` });
  }
  if (ov.disk_used_gb !== undefined) {
    kpis.push({ label: 'Disk', value: ov.disk_used_gb + ' GB' });
  }
  return kpis;
}

/**
 * Format nightly jobs for the overview widget (last 5, with status badge).
 * @param {Array} jobs — raw jobs array from API
 * @returns {Array<object>} table-ready rows
 */
function formatNightlyJobs(jobs) {
  return jobs.slice(0, 5).map(j => ({
    ...j,
    status: window.statusDot ? window.statusDot(j.status) + ' ' + j.status : j.status,
    started_at: j.started_at ? new Date(j.started_at).toLocaleString() : '',
  }));
}

/**
 * Compute mesh health percentage.
 * @param {object} ov — overview API payload
 * @returns {{value: number, max: number}}
 */
function meshHealthGauge(ov) {
  const pct = ov.mesh_total > 0
    ? Math.round((ov.mesh_online / ov.mesh_total) * 100)
    : 0;
  return { value: pct, max: 100 };
}

/**
 * Overview view factory.
 * @param {HTMLElement} container — mount target
 * @param {{api: object, store: object}} deps
 * @returns {Function} teardown callback
 */
export default function overview(container, { api, store }) {
  const scaffold = new StateScaffold(container, {
    state: 'loading',
    onRetry: () => refresh(),
  });

  const renderer = new DashboardRenderer(container, { schema: SCHEMA });

  async function refresh() {
    scaffold.state = 'loading';

    const results = await Promise.allSettled([
      api.fetchOverview(),
      api.fetchTokensDaily(),
      api.fetchTokensModels(),
      api.fetchTasksDistribution(),
      api.fetchHistory(),
      api.fetchNightlyJobs(1, 5),
    ]);

    const [ovResult, dailyResult, modelsResult, distResult, histResult, nightlyResult] = results;
    let hasData = false;

    if (ovResult.status === 'fulfilled' && !(ovResult.value instanceof Error)) {
      const ov = ovResult.value;
      renderer.setData('kpis', buildKpis(ov));
      renderer.setData('meshHealth', meshHealthGauge(ov));
      hasData = true;
    }

    if (dailyResult.status === 'fulfilled' && !(dailyResult.value instanceof Error)) {
      renderer.setData('tokenBurn', dailyResult.value.map(d => d.input + d.output));
      hasData = true;
    }

    if (modelsResult.status === 'fulfilled' && !(modelsResult.value instanceof Error)) {
      renderer.setData('modelCost', modelsResult.value);
      hasData = true;
    }

    if (distResult.status === 'fulfilled' && !(distResult.value instanceof Error)) {
      renderer.setData('taskDist', distResult.value);
      hasData = true;
    }

    if (histResult.status === 'fulfilled' && !(histResult.value instanceof Error)) {
      renderer.setData('activity', histResult.value.slice(0, 10));
      hasData = true;
    }

    if (nightlyResult.status === 'fulfilled' && !(nightlyResult.value instanceof Error)) {
      const jobs = nightlyResult.value?.jobs || [];
      if (jobs.length > 0) {
        renderer.setData('nightlyJobs', {
          columns: NIGHTLY_COLS,
          rows: formatNightlyJobs(jobs),
        });
      } else {
        console.warn('[overview] nightly jobs returned empty');
      }
      hasData = true;
    }

    if (!hasData) {
      const firstError = results.find(r => r.status === 'rejected');
      scaffold.state = 'error';
      console.warn('[overview] all fetches failed', firstError?.reason);
      return;
    }

    scaffold.state = 'ready';
  }

  refresh();
  const unsub = store.subscribe('overview', () => refresh());

  return () => {
    unsub();
    renderer.destroy();
  };
}
