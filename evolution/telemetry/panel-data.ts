import type { Metric } from '../core/types/index.js';

export interface PanelSnapshot {
  generatedAt: number;
  trends: {
    agentTokens: number[];
  };
  kpis: {
    costTodayUsd: number;
    activePlans: number;
    completionRate: number;
  };
  table: Array<{ name: string; value: number; ts: number }>;
}

export function buildPanelSnapshot(metrics: Metric[]): PanelSnapshot {
  const sorted = [...metrics].sort((left, right) => left.timestamp - right.timestamp);
  const latest = (name: string): number =>
    [...sorted].reverse().find((metric) => metric.name === name)?.value ?? 0;

  return {
    generatedAt: Date.now(),
    trends: {
      agentTokens: sorted
        .filter((metric) => metric.name === 'agent.tokens.input')
        .map((metric) => metric.value),
    },
    kpis: {
      costTodayUsd: latest('agent.cost.usd'),
      activePlans: latest('agent.plan.active_count'),
      completionRate: latest('agent.task.completion_rate'),
    },
    table: sorted.map((metric) => ({
      name: metric.name,
      value: metric.value,
      ts: metric.timestamp,
    })),
  };
}
