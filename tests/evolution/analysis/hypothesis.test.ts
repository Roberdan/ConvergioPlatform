import { describe, expect, it } from 'vitest';
import { buildHypothesis } from '../../../evolution/analysis/hypothesis.ts';
import { scoreHypothesis } from '../../../evolution/analysis/scoring-model.ts';

describe('hypothesis modeling', () => {
  it('builds a hypothesis definition with thresholds', () => {
    const hypothesis = buildHypothesis({
      hypothesis: 'Code splitting reduces JS bundle by 20%',
      targetMetric: 'bundle.js_size_bytes',
      successThreshold: -0.2,
      failureThreshold: 0.05,
    });

    expect(hypothesis.targetMetric).toBe('bundle.js_size_bytes');
    expect(hypothesis.successThreshold).toBe(-0.2);
  });

  it('scores hypotheses using blast radius weight', () => {
    const score = scoreHypothesis(20, 'SingleFile');
    expect(score.score).toBe(0.8);
    expect(score.blastRadius).toBe('SingleFile');
  });
});
