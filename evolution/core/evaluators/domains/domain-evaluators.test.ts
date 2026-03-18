import { describe, expect, it } from 'vitest';
import type { Metric } from '../../types/index.js';
import type { AggregatedPoint } from '../../../telemetry/aggregation.js';
import { LatencyEvaluator } from './latency-evaluator.js';
import { BundleEvaluator } from './bundle-evaluator.js';
import { AgentCostEvaluator } from './agent-cost-evaluator.js';
import { MeshTopologyEvaluator } from './mesh-topology-evaluator.js';
import { DbEvaluator } from './db-evaluator.js';
import { WorkloadEvaluator } from './workload-evaluator.js';

const now = Date.now();

function mk(name: string, value: number, family: Metric['family']): Metric {
  return { name, value, timestamp: now, labels: {}, family };
}

const history: AggregatedPoint[] = [];

describe('domain evaluators', () => {
  it('detects runtime latency anomalies', async () => {
    const evaluator = new LatencyEvaluator();
    const result = await evaluator.evaluate([
      mk('http.p95_latency_ms', 650, 'Runtime'),
      mk('http.p50_latency_ms', 250, 'Runtime'),
      mk('http.error_rate_pct', 3, 'Runtime'),
    ], history);

    expect(result.anomalies.some((a) => a.metric === 'http.error_rate_pct')).toBe(true);
    expect(result.opportunities.length).toBe(3);
  });

  it('detects bundle and build opportunities', async () => {
    const evaluator = new BundleEvaluator();
    const result = await evaluator.evaluate([
      mk('bundle.js_size_bytes', 1_100_000, 'Bundle'),
      mk('bundle.css_size_bytes', 300_000, 'Bundle'),
      mk('build.duration_ms', 140_000, 'Build'),
    ], history);

    expect(result.anomalies.some((a) => a.metric === 'bundle.js_size_bytes')).toBe(true);
    expect(result.anomalies.some((a) => a.metric === 'bundle.css_size_bytes')).toBe(true);
    expect(result.anomalies.some((a) => a.metric === 'build.duration_ms')).toBe(true);
  });

  it('detects agent cost and completion anomalies', async () => {
    const evaluator = new AgentCostEvaluator();
    const result = await evaluator.evaluate([
      mk('agent.cost.usd', 120, 'Agent'),
      mk('agent.task.completion_rate', 0.35, 'Agent'),
      mk('agent.tokens.input', 10000, 'Agent'),
      mk('agent.tokens.output', 8000, 'Agent'),
    ], [{ name: 'agent.cost.usd', family: 'Agent', ts: now - 60_000, avg: 60, min: 55, max: 65, count: 7 }]);

    expect(result.anomalies.length).toBeGreaterThanOrEqual(2);
    expect(result.opportunities.length).toBe(3);
  });

  it('detects mesh topology pressure', async () => {
    const evaluator = new MeshTopologyEvaluator();
    const result = await evaluator.evaluate([
      mk('mesh.sync_lag_ms', 130_000, 'Mesh'),
      mk('mesh.packet_loss_pct', 6, 'Mesh'),
      mk('mesh.peer_count', 3, 'Mesh'),
    ], history);

    expect(result.domain).toBe('mesh_topology');
    expect(result.anomalies.length).toBeGreaterThanOrEqual(2);
  });

  it('detects database pressure', async () => {
    const evaluator = new DbEvaluator();
    const result = await evaluator.evaluate([
      mk('db.query_p95_ms', 620, 'Database'),
      mk('db.connection_pool_usage', 0.92, 'Database'),
      mk('db.wal_size_bytes', 130 * 1024 * 1024, 'Database'),
    ], history);

    expect(result.anomalies.length).toBeGreaterThanOrEqual(3);
    expect(result.opportunities.length).toBe(3);
  });

  it('detects workload saturation', async () => {
    const evaluator = new WorkloadEvaluator();
    const result = await evaluator.evaluate([
      mk('workload.queue_depth', 700, 'Workload'),
      mk('workload.task_error_rate', 6, 'Workload'),
      mk('runtime.memory_mb', 8200, 'Runtime'),
      mk('runtime.cpu_pct', 93, 'Runtime'),
    ], history);

    expect(result.anomalies.some((a) => a.metric === 'runtime.memory_mb')).toBe(true);
    expect(result.anomalies.some((a) => a.metric === 'runtime.cpu_pct')).toBe(true);
    expect(result.opportunities.length).toBe(4);
  });
});
