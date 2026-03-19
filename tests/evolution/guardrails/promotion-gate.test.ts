import { describe, expect, it } from 'vitest';
import { PromotionGate } from '../../../src/ts/evolution/guardrails/promotion-gate.ts';

describe('PromotionGate', () => {
  it('allows shadow to canary when delta > 5', () => {
    const gate = new PromotionGate();
    expect(gate.promote('Shadow', 'Canary', { deltaScore: 6, hasHighAnomalies: false })).toBe(true);
  });

  it('blocks canary to bluegreen when anomalies are high', () => {
    const gate = new PromotionGate();
    expect(gate.promote('Canary', 'BlueGreen', { deltaScore: 20, hasHighAnomalies: true })).toBe(false);
  });
});
