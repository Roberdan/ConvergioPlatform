import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, rmSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import type { Metric } from '../core/types/index.js';
import { MetricStore } from './store.js';

function metric(name: string, family: Metric['family'], ts: number, value: number): Metric {
  return {
    name,
    family,
    timestamp: ts,
    value,
    labels: { env: 'test' },
  };
}

describe('MetricStore', () => {
  let dir = '';
  let dbPath = '';

  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), 'evo-store-'));
    dbPath = join(dir, 'telemetry.db');
  });

  afterEach(() => {
    rmSync(dir, { recursive: true, force: true });
  });

  it('persists and queries metrics by name and family range', () => {
    const store = new MetricStore({ dbPath });
    const now = Date.now();
    store.persist([
      metric('agent.tokens.input', 'Agent', now - 1000, 12),
      metric('agent.tokens.output', 'Agent', now - 500, 7),
      metric('runtime.loop.utilization', 'Runtime', now - 500, 0.7),
    ]);

    const queried = store.query({ family: 'Agent', from: now - 2000, to: now });
    expect(queried).toHaveLength(2);
    expect(queried.map((item) => item.name).sort()).toEqual(['agent.tokens.input', 'agent.tokens.output']);
  });

  it('aggregates points in 5-minute buckets', () => {
    const store = new MetricStore({ dbPath });
    const seed = 1_700_000_000_000;
    const base = seed - (seed % 300_000);
    store.persist([
      metric('agent.cost.usd', 'Agent', base + 30_000, 2),
      metric('agent.cost.usd', 'Agent', base + 120_000, 4),
      metric('agent.cost.usd', 'Agent', base + 240_000, 6),
    ]);

    const points = store.aggregate({
      name: 'agent.cost.usd',
      family: 'Agent',
      from: base,
      to: base + 300_000,
    });

    expect(points).toHaveLength(1);
    expect(points[0].avg).toBe(4);
    expect(points[0].min).toBe(2);
    expect(points[0].max).toBe(6);
    expect(points[0].count).toBe(3);
  });

  it('purges old rows based on retention days', () => {
    const store = new MetricStore({ dbPath });
    const now = Date.now();
    const tenDaysMs = 10 * 24 * 60 * 60 * 1000;

    store.persist([
      metric('agent.session.count', 'Agent', now - tenDaysMs, 1),
      metric('agent.session.count', 'Agent', now, 2),
    ]);

    store.purge(7);

    const rows = store.query({ name: 'agent.session.count', from: now - tenDaysMs, to: now + 1 });
    expect(rows).toHaveLength(1);
    expect(rows[0].value).toBe(2);
  });
});
