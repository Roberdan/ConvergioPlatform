import { describe, expect, it } from 'vitest';
import { MeshAnalyzer } from '../../../src/ts/evolution/runtime/mesh-analyzer.ts';

describe('MeshAnalyzer', () => {
  it('flags high mesh synchronization lag and packet loss', async () => {
    const analyzer = new MeshAnalyzer();
    const result = await analyzer.analyze([
      { name: 'mesh.sync_lag_ms', value: 130_000, timestamp: Date.now(), labels: {}, family: 'Mesh' },
      { name: 'mesh.packet_loss_pct', value: 7, timestamp: Date.now(), labels: {}, family: 'Mesh' },
    ]);

    expect(result.domain).toBe('mesh_topology');
    expect(result.anomalies.length).toBeGreaterThan(0);
  });
});
