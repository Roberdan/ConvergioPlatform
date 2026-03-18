function _stripStat(label, value, sub, signal) {
  const cls = signal === 'danger' ? ' mn-strip__value--danger' : signal === 'warn' ? ' mn-strip__value--warn' : signal === 'ok' ? ' mn-strip__value--ok' : '';
  return `<div class="mn-strip__section">` +
    `<div class="mn-strip__label mn-micro">${label}</div>` +
    `<div class="mn-strip__value${cls}">${value}</div>` +
    (sub ? `<div class="mn-strip__dim mn-micro">${sub}</div>` : '') +
    `</div><div class="mn-strip__divider"></div>`;
}

function _kpiDelta(current, previous) {
  if (previous == null) return '';
  const diff = current - previous;
  if (diff === 0) return '';
  const arrow = diff > 0 ? '▲' : '▼';
  return `${arrow}${fmt(Math.abs(diff))}`;
}

function renderKpi(d) {
  window.__kpiOverviewData = d;
  const inner = document.getElementById('kpi-inner');
  if (!inner) return;

  const online = d.mesh_online || 0, total = d.mesh_total || 0;
  const linesToday = Number(d.today_lines_changed || 0);
  const linesYesterday = d.yesterday_lines_changed != null ? Number(d.yesterday_lines_changed) : null;
  const costToday = Number(d.today_cost || 0);
  const blocked = Number(d.blocked || 0);
  const plansActive = Number(d.plans_active || 0);

  let html =
    _stripStat('ACTIVE PLANS', plansActive, `${d.plans_done || 0} done / ${d.plans_total || 0}`) +
    _stripStat('MESH NODES', `${online}/${total}`, online === total ? 'all online' : `${total - online} offline`, online === total ? 'ok' : 'warn') +
    _stripStat('TOKENS USED', fmt(d.total_tokens), `today ${fmt(d.today_tokens)}`) +
    _stripStat('COST TODAY', `$${costToday.toFixed(0)}`, '', costToday > 50 ? 'warn' : null) +
    _stripStat('LINES TODAY', fmt(linesToday), _kpiDelta(linesToday, linesYesterday)) +
    _stripStat('BLOCKED', blocked, blocked > 0 ? 'needs attention' : '', blocked > 0 ? 'danger' : null);

  // Remove trailing divider
  html = html.replace(/<div class="mn-strip__divider"><\/div>$/, '');
  inner.innerHTML = html;
}

window.renderKpi = renderKpi;
