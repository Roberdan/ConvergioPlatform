export interface HypothesisDefinition {
  id: string;
  hypothesis: string;
  targetMetric: string;
  successThreshold: number;
  failureThreshold: number;
}

export function buildHypothesis(input: {
  hypothesis: string;
  targetMetric: string;
  successThreshold: number;
  failureThreshold: number;
}): HypothesisDefinition {
  return {
    id: `H-${Date.now()}`,
    hypothesis: input.hypothesis,
    targetMetric: input.targetMetric,
    successThreshold: input.successThreshold,
    failureThreshold: input.failureThreshold,
  };
}
