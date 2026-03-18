import { loadTelemetrySnapshot } from './data-contracts.js';

function makeCard(title, value) {
  const card = document.createElement('div');
  card.className = 'telemetry-card';
  card.innerHTML = `<h3>${title}</h3><p>${value}</p>`;
  return card;
}

export async function initTelemetryPanel(container) {
  const snapshot = await loadTelemetrySnapshot('/data/telemetry-snapshot.json');
  const maranello = window.Maranello ?? {};
  const panel = snapshot.panel ?? { trends: { agentTokens: [] }, kpis: {}, table: [] };

  container.innerHTML = '';
  const graphHost = document.createElement('div');
  const kpiHost = document.createElement('div');
  const tableHost = document.createElement('div');
  kpiHost.className = 'telemetry-kpis';

  if (typeof maranello.sparkline === 'function') {
    maranello.sparkline(graphHost, panel.trends.agentTokens);
  } else if (typeof maranello.liveGraph === 'function') {
    maranello.liveGraph(graphHost, panel.trends.agentTokens);
  } else {
    const chart = document.createElement('mn-chart');
    chart.setAttribute('data-points', JSON.stringify(panel.trends.agentTokens));
    graphHost.appendChild(chart);
  }

  if (typeof maranello.kpiScorecard === 'function') {
    maranello.kpiScorecard(kpiHost, panel.kpis);
  } else {
    kpiHost.append(
      makeCard('Cost today', `$${Number(panel.kpis.costTodayUsd ?? 0).toFixed(2)}`),
      makeCard('Active plans', `${panel.kpis.activePlans ?? 0}`),
      makeCard('Completion rate', `${Math.round((panel.kpis.completionRate ?? 0) * 100)}%`),
    );
    const gauge = document.createElement('mn-gauge');
    gauge.setAttribute('value', `${panel.kpis.completionRate ?? 0}`);
    kpiHost.appendChild(gauge);
  }

  if (typeof maranello.dataTable === 'function') {
    maranello.dataTable(tableHost, panel.table);
  } else {
    const pre = document.createElement('pre');
    pre.textContent = JSON.stringify(panel.table, null, 2);
    tableHost.appendChild(pre);
  }

  container.append(graphHost, kpiHost, tableHost);
}
