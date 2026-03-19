import type { BlastRadius } from '../core/types/index.js';

export interface ProposalScore {
  score: number;
  expectedImpact: number;
  blastRadius: BlastRadius;
}

const WEIGHTS: Record<BlastRadius, number> = {
  SingleFile: 1,
  SingleRepo: 0.8,
  MultiRepo: 0.5,
  Ecosystem: 0.3,
};

export function scoreHypothesis(domainScore: number, blastRadius: BlastRadius): ProposalScore {
  const expectedImpact = (100 - domainScore) / 100;
  const score = Number((Math.max(0, expectedImpact) * WEIGHTS[blastRadius]).toFixed(3));
  return {
    score,
    expectedImpact,
    blastRadius,
  };
}
