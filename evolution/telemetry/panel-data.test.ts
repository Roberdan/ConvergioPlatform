import { describe, it, expect } from 'vitest';
import type { Metric } from '../core/types/index.js';
import { buildPanelSnapshot } from './panel-data.js';

describe('buildPanelSnapshot', () => {
  it('extracts tokens trend and KPIs from metrics', () => {
    const now = Date.now();
    const metrics: Metric[] = [
      { name: 'agent.tokens.input', value: 100, timestamp: now - 60000, labels: {}, family: 'Agent' },
      { name: 'agent.tokens.input', value: 150, timestamp: now, labels: {}, family: 'Agent' },
      { name: 'agent.cost.usd', value: 1.5, timestamp: now, labels: {}, family: 'Agent' },
      { name: 'agent.plan.active_count', value: 3, timestamp: now, labels: {}, family: 'Agent' },
      { name: 'agent.task.completion_rate', value: 0.8, timestamp: now, labels: {}, family: 'Agent' },
    ];

    const snapshot = buildPanelSnapshot(metrics);
    expect(snapshot.trends.agentTokens).toEqual([100, 150]);
    expect(snapshot.kpis.costTodayUsd).toBe(1.5);
    expect(snapshot.kpis.activePlans).toBe(3);
    expect(snapshot.kpis.completionRate).toBe(0.8);
  });
});
