export const tokenUsage = 'agent.tokens';
export const costPerTask = 'agent.cost.usd';
export const completionRate = 'agent.task.completion_rate';
export const modelSelection = 'agent.model.top';

export { AgentMetricCollector } from './collectors/agent-collector.js';
export type { AgentMetricCollectorOptions } from './collectors/agent-collector.js';
