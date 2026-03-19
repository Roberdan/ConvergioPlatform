export type ExperimentMode = 'Shadow' | 'Canary' | 'BlueGreen';

export interface PromotionInput {
  deltaScore: number;
  hasHighAnomalies: boolean;
}

export class PromotionGate {
  // promotion path: shadow -> canary -> review -> production
  promote(fromMode: ExperimentMode, toMode: ExperimentMode, input: PromotionInput): boolean {
    if (fromMode === 'Shadow' && toMode === 'Canary') {
      return input.deltaScore > 5;
    }

    if (fromMode === 'Canary' && toMode === 'BlueGreen') {
      return input.deltaScore > 10 && !input.hasHighAnomalies;
    }

    return false;
  }
}
