import { describe, it, expect } from 'vitest';
import { NaSraCanaryAdapter } from '../../../src/ts/evolution/agents/nasra-canary.js';

describe('nasra canary compatibility', () => {
  it('loads adapter from src path', () => {
    expect(NaSraCanaryAdapter).toBeDefined();
  });
});
