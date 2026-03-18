import type { AggregatedPoint } from '../../../telemetry/aggregation.js';
import type { Metric, EvaluationResult } from '../../types/index.js';
import { BaseEvaluator } from '../base-evaluator.js';
import { latestValue, opportunities } from './shared.js';

export class MeshTopologyEvaluator extends BaseEvaluator {
  readonly domain = 'mesh_topology';
  readonly metricFamilies = ['Mesh'] as const;

  protected async analyze(metrics: Metric[], _history: AggregatedPoint[]): Promise<Partial<EvaluationResult>> {
    const anomalies: EvaluationResult['anomalies'] = [];
    const syncLagMs = latestValue(metrics, 'mesh.sync_lag_ms');
    const packetLossPct = latestValue(metrics, 'mesh.packet_loss_pct');
    latestValue(metrics, 'mesh.peer_count');

    if (syncLagMs !== null && syncLagMs > 120_000) {
      anomalies.push({ metric: 'mesh.sync_lag_ms', severity: 'high', detail: `Sync lag=${syncLagMs}ms > 120000ms` });
    } else if (syncLagMs !== null && syncLagMs > 30_000) {
      anomalies.push({ metric: 'mesh.sync_lag_ms', severity: 'medium', detail: `Sync lag=${syncLagMs}ms > 30000ms` });
    }
    if (packetLossPct !== null && packetLossPct > 5) {
      anomalies.push({ metric: 'mesh.packet_loss_pct', severity: 'medium', detail: `Packet loss=${packetLossPct}% > 5%` });
    }

    return {
      anomalies,
      opportunities: anomalies.length
        ? opportunities(
            this.domain,
            ['Reduce sync batch interval', 'Add peer health check', 'Enable Tailscale direct route'],
            '-10% to -30% mesh synchronization lag',
          )
        : [],
    };
  }
}
