import { describe, it, expect } from 'vitest';
import type { Metric, Proposal, Experiment } from './index.js';
import type { PlatformAdapter } from './adapter.js';
import {
  MLDAdapter,
  ClaudeConfigAdapter,
  DashboardAdapter,
  TemplateAdapter,
} from '../../adapters/index.js';

describe('Evolution Engine Types', () => {
  it('Metric type is well-formed', () => {
    const m: Metric = {
      name: 'p95_latency',
      value: 42,
      timestamp: Date.now(),
      labels: { service: 'mesh' },
      family: 'Runtime',
    };
    expect(m.name).toBe('p95_latency');
    expect(m.family).toBe('Runtime');
  });

  it('Proposal requires hypothesis and blast radius', () => {
    const p: Proposal = {
      id: 'EVO-20260318-0001',
      hypothesis: 'Caching reduces p95 by 30%',
      targetMetric: 'p95_latency',
      expectedDelta: { min: -0.2, max: -0.4 },
      successCriteria: 'p95 < 30ms',
      failureCriteria: 'p95 > 50ms or error_rate > 1%',
      blastRadius: 'SingleRepo',
      sourceType: 'Internal',
      status: 'Draft',
    };
    expect(p.blastRadius).toBe('SingleRepo');
    expect(p.hypothesis).toBeTruthy();
    expect(p.status).toBe('Draft');
  });

  it('Experiment links proposalId and tracks lifecycle', () => {
    const e: Experiment = {
      id: 'exp-001',
      proposalId: 'EVO-20260318-0001',
      mode: 'Canary',
      startedAt: Date.now(),
      beforeMetrics: [],
      afterMetrics: [],
    };
    expect(e.mode).toBe('Canary');
    expect(e.completedAt).toBeUndefined();
  });

  it('PlatformAdapter contract is structurally satisfied', () => {
    const mockAdapter: PlatformAdapter = {
      name: 'test-adapter',
      collectMetrics: async () => [],
      runCanary: async () => ({
        confidence: 0.95,
        pValue: 0.03,
        recommendation: 'Apply',
        delta: -0.31,
        sideEffects: [],
      }),
      openPR: async () => ({ prUrl: 'https://github.com/test/pr/1', prNumber: 1 }),
      rollback: async () => {},
      healthCheck: async () => ({ healthy: true, details: 'ok' }),
    };
    expect(mockAdapter.name).toBe('test-adapter');
  });
});

describe('PlatformAdapter implementations', () => {
  it('MLDAdapter satisfies PlatformAdapter', () => {
    const adapter: PlatformAdapter = new MLDAdapter('/tmp/test-mld-repo');
    expect(adapter.name).toBe('mld');
    expect(typeof adapter.collectMetrics).toBe('function');
    expect(typeof adapter.runCanary).toBe('function');
    expect(typeof adapter.openPR).toBe('function');
    expect(typeof adapter.healthCheck).toBe('function');
  });

  it('ClaudeConfigAdapter satisfies PlatformAdapter', () => {
    const adapter: PlatformAdapter = new ClaudeConfigAdapter();
    expect(adapter.name).toBe('claude-config');
  });

  it('DashboardAdapter satisfies PlatformAdapter', () => {
    const adapter: PlatformAdapter = new DashboardAdapter('http://localhost:3000');
    expect(adapter.name).toBe('dashboard');
  });

  it('TemplateAdapter satisfies PlatformAdapter', () => {
    const adapter: PlatformAdapter = new TemplateAdapter('test-target');
    expect(adapter.name).toBe('template');
  });
});
