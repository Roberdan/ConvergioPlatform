/**
 * EscalationMetricCollector — tracks agent escalation patterns from channel
 * interactions and feeds them to the evolution engine as Metric + Proposal data.
 *
 * Why: channel interactions produce approval signals that reveal which agents
 * can safely gain more autonomy and which need boundary review.
 */

import type { Metric, MetricCollector, Proposal, ExperimentResult } from '../core/types/index.js';
import type { PlatformAdapter } from '../core/types/adapter.js';

/** A single escalation event emitted by a channel interaction. */
export interface EscalationEvent {
  /** Stable agent identifier */
  agent_id: string;
  /** Logical category of the action requiring approval */
  event_type: string;
  /** Channel where the escalation was presented (e.g. 'telegram', 'slack') */
  channel: string;
  /** Unix epoch milliseconds when the escalation was raised */
  timestamp: number;
  /** Whether the escalation was approved by a human operator */
  was_approved: boolean;
  /** Time from escalation raised to decision, in milliseconds */
  response_time_ms: number;
}

/** Per-agent aggregated statistics derived from recorded events. */
export interface AgentEscalationStats {
  agent_id: string;
  escalation_rate_per_hour: number;
  approval_rate: number;
  avg_response_time_ms: number;
  event_count: number;
}

const APPROVAL_RATE_HIGH_THRESHOLD = 0.95;
const ESCALATION_RATE_HIGH_THRESHOLD = 10; // events per hour
const MIN_EVENTS_FOR_PROPOSAL = 5; // avoid noisy signals from sparse data

/**
 * Collector that tracks escalation events from channel adapters, computes
 * per-agent statistics, and generates evolution Proposals when patterns emerge.
 *
 * Implements both MetricCollector (engine polling) and PlatformAdapter
 * (canary/PR lifecycle hooks).
 */
export class EscalationMetricCollector implements MetricCollector, PlatformAdapter {
  readonly id = 'escalation-collector';
  readonly name = 'escalation-collector';
  readonly families = ['Agent'] as const;

  private readonly events: EscalationEvent[] = [];

  /** Record a new escalation event from a channel interaction. */
  record(event: EscalationEvent): void {
    this.events.push(event);
  }

  /**
   * PlatformAdapter contract — delegates to collect().
   * Why: PlatformAdapter.collectMetrics and MetricCollector.collect are the same
   * operation with different method names; bridging here keeps both interfaces satisfied.
   */
  async collectMetrics(): Promise<Metric[]> {
    return this.collect();
  }

  /** Collect per-agent escalation metrics for the evolution engine. */
  async collect(): Promise<Metric[]> {
    if (this.events.length === 0) return [];
    const stats = this.computeStats();
    const now = Date.now();
    const metrics: Metric[] = [];
    for (const s of stats) {
      const labels = { agent_id: s.agent_id };
      metrics.push(buildMetric('escalation.escalation_rate', s.escalation_rate_per_hour, now, labels));
      metrics.push(buildMetric('escalation.approval_rate', s.approval_rate, now, labels));
      metrics.push(buildMetric('escalation.avg_response_time_ms', s.avg_response_time_ms, now, labels));
      metrics.push(buildMetric('escalation.event_count', s.event_count, now, labels));
    }
    return metrics;
  }

  /** Generate evolution Proposals based on observed escalation patterns. */
  generateProposals(): Proposal[] {
    const proposals: Proposal[] = [];
    const now = Date.now();
    for (const s of this.computeStats()) {
      if (s.event_count < MIN_EVENTS_FOR_PROPOSAL) continue;
      if (s.approval_rate > APPROVAL_RATE_HIGH_THRESHOLD) {
        proposals.push(buildProposal(
          makeId(now, proposals.length),
          `increase_autonomy for ${s.agent_id}`,
          `Agent ${s.agent_id} has ${(s.approval_rate * 100).toFixed(1)}% approval rate ` +
            `over ${s.event_count} escalations. Consider granting higher autonomy.`,
          now,
        ));
      }
      if (s.escalation_rate_per_hour > ESCALATION_RATE_HIGH_THRESHOLD) {
        proposals.push(buildProposal(
          makeId(now, proposals.length),
          `review_decision_boundaries for ${s.agent_id}`,
          `Agent ${s.agent_id} is escalating at ${s.escalation_rate_per_hour.toFixed(1)}/hour, ` +
            `exceeding the ${ESCALATION_RATE_HIGH_THRESHOLD}/hour threshold. Review its decision boundaries.`,
          now,
        ));
      }
    }
    // Propose auto-approval when an event type is always approved
    for (const [eventType, typeEvents] of this.groupByEventType()) {
      if (typeEvents.length < MIN_EVENTS_FOR_PROPOSAL) continue;
      if (typeEvents.every((e) => e.was_approved)) {
        proposals.push(buildProposal(
          makeId(now, proposals.length),
          `auto_approve_event: ${eventType}`,
          `Event type '${eventType}' has been approved in all ${typeEvents.length} recorded ` +
            `escalations. Consider configuring automatic approval to reduce operator burden.`,
          now,
        ));
      }
    }
    return proposals;
  }

  async healthCheck(): Promise<{ healthy: boolean; details: string }> {
    return { healthy: true, details: `escalation-collector active; ${this.events.length} events recorded` };
  }

  async runCanary(proposal: Proposal): Promise<ExperimentResult> {
    // Escalation pattern changes require human review before canary deployment.
    void proposal;
    return { confidence: 0, pValue: 1, recommendation: 'Inconclusive', delta: 0, sideEffects: [] };
  }

  async openPR(proposal: Proposal): Promise<{ prUrl: string; prNumber: number }> {
    throw new Error(`openPR not implemented for escalation proposal ${proposal.id}`);
  }

  async rollback(experimentId: string): Promise<void> {
    void experimentId; // no persistent state to revert for in-memory collector
  }

  private computeStats(): AgentEscalationStats[] {
    const byAgent = new Map<string, EscalationEvent[]>();
    for (const e of this.events) {
      const list = byAgent.get(e.agent_id) ?? [];
      list.push(e);
      byAgent.set(e.agent_id, list);
    }
    return [...byAgent.entries()].map(([id, evts]) => this.statsForAgent(id, evts));
  }

  private statsForAgent(agent_id: string, agentEvents: EscalationEvent[]): AgentEscalationStats {
    const count = agentEvents.length;
    const approved = agentEvents.filter((e) => e.was_approved).length;
    const totalResponseTime = agentEvents.reduce((sum, e) => sum + e.response_time_ms, 0);
    const timestamps = agentEvents.map((e) => e.timestamp);
    const spanMs = Math.max(Math.max(...timestamps) - Math.min(...timestamps), 1);
    const spanHours = spanMs / 3_600_000;
    return {
      agent_id,
      // floor window at 1 minute to avoid inflated rates for near-simultaneous events
      escalation_rate_per_hour: count / Math.max(spanHours, 1 / 60),
      approval_rate: approved / count,
      avg_response_time_ms: totalResponseTime / count,
      event_count: count,
    };
  }

  private groupByEventType(): Map<string, EscalationEvent[]> {
    const map = new Map<string, EscalationEvent[]>();
    for (const e of this.events) {
      const list = map.get(e.event_type) ?? [];
      list.push(e);
      map.set(e.event_type, list);
    }
    return map;
  }
}

function buildMetric(
  name: string, value: number, timestamp: number, labels: Record<string, string> = {},
): Metric {
  return { name, value, timestamp, labels, family: 'Agent' };
}

function makeId(now: number, index: number): string {
  return `EVO-${new Date(now).toISOString().slice(0, 10).replace(/-/g, '')}-${String(index + 1).padStart(4, '0')}`;
}

function buildProposal(id: string, title: string, description: string, now: number): Proposal {
  return {
    id, title, description,
    failureCriteria: 'approval_rate drops below 0.70 after change',
    rollbackStrategy: 'revert autonomy configuration to previous snapshot',
    estimatedGain: 'reduced operator interruptions, faster decision throughput',
    confidence: 0.8,
    createdAt: now,
    blastRadius: 'SingleRepo',
    sourceType: 'Internal',
    status: 'Draft',
    targetAdapter: 'escalation-collector',
  };
}
