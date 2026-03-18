import { describe, it, expect } from 'vitest';
import { ModelIntelligence } from '../../../src/ts/evolution/agents/model-intelligence.js';
import { BenchmarkRunner } from '../../../src/ts/evolution/agents/model-benchmark.js';

describe('model intelligence compatibility', () => {
  it('loads model intelligence and benchmark runner', () => {
    expect(ModelIntelligence).toBeDefined();
    expect(BenchmarkRunner).toBeDefined();
  });
});
