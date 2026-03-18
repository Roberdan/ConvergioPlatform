import { describe, expect, it } from 'vitest';
import { collectMldTelemetry, toMetrics } from '../../../src/ts/evolution/canary/mld-telemetry-feed';

describe('mld-telemetry-feed', () => {
  it('returns snapshot with required telemetry fields', () => {
    const snapshot = collectMldTelemetry();
    expect(snapshot.buildTime).toBeTypeOf('number');
    expect(snapshot.bundleSize).toBeTypeOf('number');
    expect(snapshot.testPassRate).toBeTypeOf('number');
    expect(snapshot.a11yScore).toBeTypeOf('number');
    expect(snapshot.coverage).toBeTypeOf('number');
    expect(snapshot.collectedAt).toBeTypeOf('number');
  });

  it('maps telemetry to metric tuples', () => {
    const metrics = toMetrics({
      buildTime: 11,
      bundleSize: 220000,
      testPassRate: 0.92,
      a11yScore: 0.98,
      coverage: 0.81,
      collectedAt: Date.now(),
    });

    expect(metrics.map((m) => m.name)).toEqual([
      'mld.buildTime',
      'mld.bundleSize',
      'mld.testPassRate',
      'mld.a11yScore',
      'mld.coverage',
    ]);
  });
});
