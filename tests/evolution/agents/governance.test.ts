import { describe, it, expect } from 'vitest';
import { AgentGovernance } from '../../../src/ts/evolution/agents/governance.js';
import { AgentValidator } from '../../../src/ts/evolution/agents/validators.js';

describe('governance compatibility', () => {
  it('loads governance and validator from src/ts path', () => {
    expect(AgentGovernance).toBeDefined();
    expect(AgentValidator).toBeDefined();
  });
});
