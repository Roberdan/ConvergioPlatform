export async function initCadenceView(container) {
  const response = await fetch('/data/cadence-status.json', { cache: 'no-store' });
  const status = response.ok ? await response.json() : {};

  container.innerHTML = '';

  const trend = document.createElement('mn-chart');
  trend.setAttribute('data-points', JSON.stringify(status.trend ?? [status.lastDeltaScore ?? 0]));

  const findings = document.createElement('mn-data-table');
  findings.setAttribute(
    'data-rows',
    JSON.stringify([
      { label: 'Last run', value: status.lastRunTimestamp ?? 'n/a' },
      { label: 'Next run', value: status.nextScheduledRun ?? 'n/a' },
      { label: 'Last delta score', value: status.lastDeltaScore ?? 0 },
    ]),
  );

  container.append(trend, findings);
}
