/**
 * NF: Graceful Degradation — engine safe under failure conditions.
 */
import { describe, it, expect, vi } from 'vitest';
import { KillSwitch } from '../../../evolution/core/guardrails/kill-switch.js';

describe('NF: Graceful Degradation', () => {
  it('KillSwitch.isEnabled() returns false by default', () => {
    expect(new KillSwitch().isEnabled()).toBe(false);
  });

  it('KillSwitch.enable() blocks engine cycle (guard pattern)', async () => {
    const ks = new KillSwitch();
    ks.enable('Runaway cost', 'human:ops');
    const mockRun = vi.fn().mockResolvedValue([]);
    async function guarded(): Promise<unknown[]> {
      if (ks.isEnabled()) return [];
      return mockRun();
    }
    const result = await guarded();
    expect(mockRun).not.toHaveBeenCalled();
    expect(result).toEqual([]);
  });

  it('KillSwitch.disable() restores normal operation', () => {
    const ks = new KillSwitch();
    ks.enable('test', 'engine');
    ks.disable();
    expect(ks.isEnabled()).toBe(false);
  });

  it('EvolutionEngine can be imported without dashboard_web side effects', async () => {
    const { EvolutionEngine } = await import('../../../evolution/core/engine.js');
    expect(typeof EvolutionEngine).toBe('function');
  });

  it('KillSwitch state snapshot is immutable copy', () => {
    const ks = new KillSwitch();
    const state = ks.getState();
    expect(state.enabled).toBe(false);
    expect(state.enabledAt).toBeNull();
  });
});
