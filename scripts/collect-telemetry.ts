import { mkdirSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { TelemetrySdk } from '../evolution/telemetry/sdk.js';
import { AgentMetricCollector } from '../evolution/telemetry/collectors/agent-collector.js';
import { MetricStore } from '../evolution/telemetry/store.js';
import { buildPanelSnapshot } from '../evolution/telemetry/panel-data.js';

async function main(): Promise<void> {
  const store = new MetricStore();
  const sdk = new TelemetrySdk({
    sink: (snapshot) => {
      store.persist(snapshot.metrics);
      store.purge(30);
    },
  });

  sdk.register(new AgentMetricCollector());
  const snapshot = await sdk.collect();
  const panelSnapshot = buildPanelSnapshot(snapshot.metrics);

  const outputPath = resolve(process.cwd(), 'data/telemetry-snapshot.json');
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, JSON.stringify({ ...snapshot, panel: panelSnapshot }, null, 2));
  process.stdout.write(`wrote ${outputPath}\n`);
}

void main();
