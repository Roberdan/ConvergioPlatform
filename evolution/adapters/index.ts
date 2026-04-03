/**
 * Platform adapters barrel — import all concrete PlatformAdapter implementations here.
 */

export { MLDAdapter } from './mld-adapter.js';
export { ClaudeConfigAdapter } from './claude-adapter.js';
export { DashboardAdapter } from './dashboard-adapter.js';
export { AgentMetricCollector } from './agent-telemetry-collector.js';
export { EscalationMetricCollector } from './escalation-collector.js';
export type { EscalationEvent, AgentEscalationStats } from './escalation-collector.js';
