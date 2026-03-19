import { describe, expect, it } from 'vitest';
import { PREnforcer } from '../../../src/ts/evolution/guardrails/pr-only-enforcer.ts';

describe('PREnforcer validate', () => {
  it('auto-approves single repo with high confidence', () => {
    const gate = new PREnforcer();
    const result = gate.validate({
      id: 'p1',
      title: 't',
      description: 'd',
      blastRadius: 'SingleRepo',
      sourceType: 'Internal',
      status: 'Draft',
      targetAdapter: 'x',
      failureCriteria: 'err>1',
      rollbackStrategy: 'git revert',
      estimatedGain: '10%',
      confidence: 0.9,
    });
    expect(result.allowed).toBe(true);
  });

  it('blocks ecosystem with explicit rejection reason', () => {
    const gate = new PREnforcer();
    const proposal = {
      id: 'p2',
      title: 't',
      description: 'd',
      blastRadius: 'Ecosystem' as const,
      sourceType: 'Internal' as const,
      status: 'Draft' as const,
      targetAdapter: 'x',
      failureCriteria: 'x',
      rollbackStrategy: 'y',
      estimatedGain: '2%',
      confidence: 0.5,
    };
    const result = gate.validate(proposal);
    expect(result.allowed).toBe(false);
    expect(result.reason).toContain('board review');
    expect(proposal.status).toBe('Rejected');
  });
});
