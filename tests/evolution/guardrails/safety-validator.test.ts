import { describe, expect, it } from 'vitest';
import { SafetyValidator } from '../../../src/ts/evolution/guardrails/safety-validator.ts';

describe('SafetyValidator', () => {
  it('fails when rollback strategy missing', () => {
    const validator = new SafetyValidator();
    const report = validator.validate({
      id: 'p1',
      title: 't',
      description: 'd',
      blastRadius: 'SingleRepo',
      sourceType: 'Internal',
      status: 'Draft',
      targetAdapter: 'x',
      failureCriteria: 'err',
      rollbackStrategy: '',
      estimatedGain: '5%',
      confidence: 0.9,
    });
    expect(report.safe).toBe(false);
    expect(report.blockers.join(' ')).toContain('rollback');
  });
});
