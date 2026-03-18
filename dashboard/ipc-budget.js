// ipc-budget.js — Budget intelligence panel for dashboard
// Uses Maranello DS components (mn-widget, mn-badge)

async function renderIpcBudget(container) {
  if (!container) return;
  const data = await fetchJson('/api/ipc/budget');
  if (!data || !data.budgets) {
    container.innerHTML = '<mn-widget title="Budget"><p>No budget data</p></mn-widget>';
    return;
  }
  const html = data.budgets.map(b => {
    const status = b.status || {};
    const pct = status.usage_pct || 0;
    const level = pct >= 95 ? 'critical' : pct >= 85 ? 'high' : pct >= 70 ? 'warn' : 'ok';
    const color = level === 'critical' ? '#ef4444' : level === 'high' ? '#f59e0b' : level === 'warn' ? '#eab308' : '#22c55e';
    const gaugeWidth = Math.min(pct, 100);
    return `
      <mn-widget title="${b.subscription}" class="mn-card ipc-budget-card">
        <div class="budget-gauge" style="margin:8px 0">
          <div class="gauge-track" style="background:#1e293b;border-radius:6px;height:12px;overflow:hidden">
            <div class="gauge-fill" style="width:${gaugeWidth}%;background:${color};height:100%;transition:width 0.5s"></div>
          </div>
          <div style="display:flex;justify-content:space-between;font-size:11px;margin-top:4px">
            <span>${pct.toFixed(0)}% used</span>
            <mn-badge variant="${level}">${level.toUpperCase()}</mn-badge>
          </div>
        </div>
        <div class="budget-details" style="display:grid;grid-template-columns:1fr 1fr;gap:4px;font-size:12px">
          <span>Budget: $${(status.budget_usd||0).toFixed(2)}</span>
          <span>Spent: $${(status.total_spent||0).toFixed(2)}</span>
          <span>Remaining: $${(status.remaining_budget||0).toFixed(2)}</span>
          <span>Projected: $${(status.projected_total||0).toFixed(2)}</span>
        </div>
        ${b.alert ? `<div class="mn-alert" style="margin-top:6px;padding:4px 8px;border-radius:4px;background:${color}22;color:${color};font-size:11px">${b.alert.message}</div>` : ''}
      </mn-widget>`;
  }).join('');
  container.innerHTML = html || '<p>No subscriptions configured</p>';
}

window.renderIpcBudget = renderIpcBudget;
