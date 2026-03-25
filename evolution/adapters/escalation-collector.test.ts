/**
 * Tests for EscalationMetricCollector — TDD RED phase first.
 * Verifies metric computation, proposal generation thresholds, and edge cases.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { EscalationMetricCollector } from './escalation-collector.js';
import type { EscalationEvent } from './escalation-collector.js';

// Helper to build an escalation event with defaults
function makeEvent(overrides: Partial<EscalationEvent> = {}): EscalationEvent {
  return {
    agent_id: 'agent-alpha',
    event_type: 'deploy_approval',
    channel: 'telegram',
    timestamp: Date.now(),
    was_approved: true,
    response_time_ms: 500,
    ...overrides,
  };
}

describe('EscalationMetricCollector', () => {
  let collector: EscalationMetricCollector;

  beforeEach(() => {
    collector = new EscalationMetricCollector();
  });

  // ── MetricCollector contract ────────────────────────────────────────────────

  it('has stable id and families', () => {
    expect(collector.id).toBe('escalation-collector');
    expect(collector.families).toContain('Agent');
  });

  // ── metric computation ──────────────────────────────────────────────────────

  it('collect() returns empty array when no events recorded', async () => {
    const metrics = await collector.collect();
    expect(metrics).toHaveLength(0);
  });

  it('computes escalation_rate per agent from recorded events', async () => {
    const now = Date.now();
    // 5 events in the last 10 minutes → 0.5/min → 30/hour
    for (let i = 0; i < 5; i++) {
      collector.record(makeEvent({ agent_id: 'agent-beta', timestamp: now - i * 60_000 }));
    }
    const metrics = await collector.collect();
    const rate = metrics.find(
      (m) => m.name === 'escalation.escalation_rate' && m.labels['agent_id'] === 'agent-beta',
    );
    expect(rate).toBeDefined();
    // 5 events / (time window in hours) — value must be positive
    expect(rate!.value).toBeGreaterThan(0);
  });

  it('computes approval_rate as fraction of approved events', async () => {
    collector.record(makeEvent({ agent_id: 'agent-gamma', was_approved: true }));
    collector.record(makeEvent({ agent_id: 'agent-gamma', was_approved: true }));
    collector.record(makeEvent({ agent_id: 'agent-gamma', was_approved: false }));

    const metrics = await collector.collect();
    const approval = metrics.find(
      (m) => m.name === 'escalation.approval_rate' && m.labels['agent_id'] === 'agent-gamma',
    );
    expect(approval).toBeDefined();
    expect(approval!.value).toBeCloseTo(2 / 3, 5);
  });

  it('computes avg_response_time across all events for an agent', async () => {
    collector.record(makeEvent({ agent_id: 'agent-delta', response_time_ms: 100 }));
    collector.record(makeEvent({ agent_id: 'agent-delta', response_time_ms: 300 }));

    const metrics = await collector.collect();
    const avg = metrics.find(
      (m) =>
        m.name === 'escalation.avg_response_time_ms' && m.labels['agent_id'] === 'agent-delta',
    );
    expect(avg).toBeDefined();
    expect(avg!.value).toBeCloseTo(200, 1);
  });

  // ── proposal generation ────────────────────────────────────────────────────

  it('generates increase_autonomy proposal when approval_rate > 0.95', () => {
    // 20 approved events → 100% approval rate
    for (let i = 0; i < 20; i++) {
      collector.record(makeEvent({ agent_id: 'agent-epsilon', was_approved: true }));
    }
    const proposals = collector.generateProposals();
    const found = proposals.find(
      (p) =>
        p.title.includes('increase_autonomy') && p.targetAdapter === 'escalation-collector',
    );
    expect(found).toBeDefined();
    expect(found!.status).toBe('Draft');
    expect(found!.blastRadius).toBe('SingleRepo');
  });

  it('generates review_decision_boundaries proposal when escalation_rate > 10/hour', () => {
    const now = Date.now();
    // 12 events within 1 hour window
    for (let i = 0; i < 12; i++) {
      collector.record(
        makeEvent({ agent_id: 'agent-zeta', timestamp: now - i * 60_000, was_approved: false }),
      );
    }
    const proposals = collector.generateProposals();
    const found = proposals.find(
      (p) =>
        p.title.includes('review_decision_boundaries') &&
        p.targetAdapter === 'escalation-collector',
    );
    expect(found).toBeDefined();
  });

  it('generates auto_approve_event proposal when an event_type is always approved', () => {
    for (let i = 0; i < 5; i++) {
      collector.record(makeEvent({ event_type: 'health_check', was_approved: true }));
    }
    const proposals = collector.generateProposals();
    const found = proposals.find(
      (p) =>
        p.title.includes('auto_approve_event') && p.description.includes('health_check'),
    );
    expect(found).toBeDefined();
  });

  it('does not generate any proposal when all metrics are below thresholds', () => {
    // 3 events, mixed approval → approval_rate ~0.67, low rate
    collector.record(makeEvent({ agent_id: 'agent-eta', was_approved: true }));
    collector.record(makeEvent({ agent_id: 'agent-eta', was_approved: true }));
    collector.record(makeEvent({ agent_id: 'agent-eta', was_approved: false }));

    const proposals = collector.generateProposals();
    // Approval rate 0.67 < 0.95; escalation count 3 < 10/hour threshold
    expect(proposals).toHaveLength(0);
  });

  // ── PlatformAdapter stub methods ────────────────────────────────────────────

  it('healthCheck returns healthy:true', async () => {
    const result = await collector.healthCheck();
    expect(result.healthy).toBe(true);
    expect(typeof result.details).toBe('string');
  });

  it('runCanary returns Inconclusive result without throwing', async () => {
    const now = Date.now();
    const proposal = {
      id: 'EVO-20260325-0001',
      title: 'increase_autonomy for agent-theta',
      description: 'Agent has >95% approval rate',
      failureCriteria: 'approval_rate drops below 0.80',
      rollbackStrategy: 'revert autonomy flag',
      estimatedGain: '+20% throughput',
      confidence: 0.9,
      createdAt: now,
      blastRadius: 'SingleRepo' as const,
      sourceType: 'Internal' as const,
      status: 'Draft' as const,
      targetAdapter: 'escalation-collector',
    };
    const result = await collector.runCanary(proposal);
    expect(result.recommendation).toBe('Inconclusive');
  });
});
