export async function initCadenceWidget(container) {
  const response = await fetch('/data/cadence-status.json', { cache: 'no-store' });
  const status = response.ok ? await response.json() : {};
  const maranello = window.Maranello ?? {};

  container.innerHTML = '';
  const timestamps = document.createElement('div');
  timestamps.innerHTML = `<p>Last run: ${status.lastRunTimestamp ?? 'n/a'}</p><p>Next run: ${status.nextScheduledRun ?? 'n/a'}</p>`;

  const scoreHost = document.createElement('div');
  if (typeof maranello.kpiScorecard === 'function') {
    maranello.kpiScorecard(scoreHost, { deltaScore: status.lastDeltaScore ?? 0 });
  } else {
    scoreHost.textContent = `Delta score: ${status.lastDeltaScore ?? 0}`;
  }

  container.append(timestamps, scoreHost);
}
