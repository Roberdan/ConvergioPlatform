import { describe, expect, it, vi } from 'vitest';

const { spawnSyncMock } = vi.hoisted(() => ({ spawnSyncMock: vi.fn() }));
vi.mock('child_process', () => ({ spawnSync: spawnSyncMock }));

import { AuditTrail } from '../../../src/ts/evolution/guardrails/audit-trail.ts';

describe('AuditTrail', () => {
  it('logs and queries entries', () => {
    spawnSyncMock.mockReturnValue({ status: 0, stdout: 'id|1|approve|p1|human|ok|SingleRepo\n', stderr: '' });
    const trail = new AuditTrail('/tmp/test.db');
    trail.log('approve', 'p1', 'human', 'ok', 'SingleRepo');
    const rows = trail.query({ proposalId: 'p1' });
    expect(rows[0]?.action).toBe('approve');
  });
});
