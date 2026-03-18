export const telemetryContract = {
  trends: ['agentTokens'],
  kpis: ['costTodayUsd', 'activePlans', 'completionRate'],
  table: ['name', 'value', 'ts'],
};

export async function loadTelemetrySnapshot(source = '/data/telemetry-snapshot.json') {
  const response = await fetch(source, { cache: 'no-store' });
  if (!response.ok) {
    throw new Error(`Unable to load telemetry snapshot: ${response.status}`);
  }
  return response.json();
}
