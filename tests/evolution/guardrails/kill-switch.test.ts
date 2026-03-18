import { describe, expect, it, vi } from 'vitest';

const { spawnSyncMock } = vi.hoisted(() => ({ spawnSyncMock: vi.fn() }));
vi.mock('child_process', () => ({ spawnSync: spawnSyncMock }));

import { KillSwitch } from '../../../src/ts/evolution/guardrails/kill-switch.ts';

describe('KillSwitch', () => {
  it('reads enabled flag from config', () => {
    spawnSyncMock.mockReturnValue({ status: 0, stdout: 'true\n', stderr: '' });
    const sw = new KillSwitch('/tmp/test.db', () => 0);
    expect(sw.isEnabled()).toBe(true);
  });

  it('writes disable and enable operations', () => {
    spawnSyncMock.mockReturnValue({ status: 0, stdout: '', stderr: '' });
    const sw = new KillSwitch('/tmp/test.db', () => 0);
    sw.disable('manual');
    sw.enable();
    expect(spawnSyncMock).toHaveBeenCalled();
  });
});
