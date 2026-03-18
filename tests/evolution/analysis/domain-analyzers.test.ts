import { describe, expect, it } from 'vitest';
import { createDomainAnalyzers } from '../../../evolution/analysis/domain-analyzers.ts';
import { OutcomeTracker } from '../../../evolution/analysis/outcome-tracker.ts';
import { mkdtempSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

describe('domain analyzers compatibility', () => {
  it('exposes five analyzers and evaluates runtime metrics', async () => {
    const analyzers = createDomainAnalyzers();
    const result = await analyzers.runtime.analyze([
      { name: 'http.p95_latency_ms', value: 700, timestamp: Date.now(), labels: {}, family: 'Runtime' },
      { name: 'http.p50_latency_ms', value: 240, timestamp: Date.now(), labels: {}, family: 'Runtime' },
    ]);
    expect(result.anomalies.length).toBeGreaterThan(0);
  });

  it('tracks outcome effectiveness delta', () => {
    const dir = mkdtempSync(join(tmpdir(), 'evo-outcome-'));
    const tracker = new OutcomeTracker(join(dir, 'outcomes.db'));
    const summary = tracker.record(
      'proposal-x',
      { domain: 'bundle', anomalies: [], opportunities: [], score: 20 },
      { domain: 'bundle', anomalies: [], opportunities: [], score: 60 },
    );

    expect(summary.effectivenessDelta).toBe(40);
    expect(summary.improved).toBe(true);
  });
});
