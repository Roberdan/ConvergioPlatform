import { describe, it, expect } from 'vitest';
import { AgentProposalGenerator } from '../../../src/ts/evolution/agents/proposal-generator.js';

describe('proposal generator compatibility', () => {
  it('loads through src/ts path', () => {
    expect(new AgentProposalGenerator()).toBeDefined();
  });
});
