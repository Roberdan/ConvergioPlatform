import { describe, expect, it, vi } from 'vitest';

const { spawnSyncMock } = vi.hoisted(() => ({ spawnSyncMock: vi.fn() }));
vi.mock('child_process', () => ({ spawnSync: spawnSyncMock }));

import { RateLimiter } from '../../../src/ts/evolution/guardrails/rate-limiter.ts';

describe('RateLimiter', () => {
  it('blocks when proposals today exceed budget', () => {
    spawnSyncMock
      .mockReturnValueOnce({ status: 0, stdout: '10\n', stderr: '' })
      .mockReturnValueOnce({ status: 0, stdout: '5\n', stderr: '' });

    const limiter = new RateLimiter('/tmp/test.db');
    expect(limiter.checkProposal()).toBe(false);
  });

  it('allows when token usage below daily budget', () => {
    spawnSyncMock
      .mockReturnValueOnce({ status: 0, stdout: '20000\n', stderr: '' })
      .mockReturnValueOnce({ status: 0, stdout: '50000\n', stderr: '' });

    const limiter = new RateLimiter('/tmp/test.db');
    expect(limiter.checkTokenBudget(1000)).toBe(true);
  });
});
