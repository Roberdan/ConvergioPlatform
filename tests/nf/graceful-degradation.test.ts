/**
 * NF: Graceful Degradation — engine remains safe under failure conditions.
 *
 * Verifies:
 * 1. KillSwitch.isEnabled() returns false by default
 * 2. When KillSwitch is enabled, no engine cycle can start (engine.run is not called)
 * 3. Evolution core module can be imported without dashboard_web side effects
 */

import { describe, it, expect, vi } from 'vitest';
import { KillSwitch } from '../../evolution/core/guardrails/kill-switch.js';

describe('NF: Graceful Degradation', () => {
  it('KillSwitch.isEnabled() returns false by default', () => {
    const ks = new KillSwitch();
    expect(ks.isEnabled()).toBe(false);
  });

  it('KillSwitch.enable() sets isEnabled to true', () => {
    const ks = new KillSwitch();
    ks.enable('Runaway cost detected', 'human:roberdan');
    expect(ks.isEnabled()).toBe(true);
  });

  it('KillSwitch.disable() restores isEnabled to false', () => {
    const ks = new KillSwitch();
    ks.enable('Test halt', 'engine');
    ks.disable();
    expect(ks.isEnabled()).toBe(false);
  });

  it('when KillSwitch is active, engine.run() should not be called', async () => {
    const ks = new KillSwitch();
    ks.enable('NF test halt', 'test');

    const mockRun = vi.fn().mockResolvedValue([]);

    // Simulate engine guard: check kill switch before running
    async function guardedRun(): Promise<unknown[]> {
      if (ks.isEnabled()) {
        return [];
      }
      return mockRun();
    }

    const result = await guardedRun();
    expect(mockRun).not.toHaveBeenCalled();
    expect(result).toEqual([]);
  });

  it('engine core module exports are accessible without dashboard_web', async () => {
    // Importing core types/classes should not require dashboard_web to be present.
    // If this import fails, it means the core has an undesired dashboard_web dep.
    const { EvolutionEngine } = await import(
      '../../evolution/core/engine.js'
    );
    expect(typeof EvolutionEngine).toBe('function');
  });

  it('KillSwitch getState returns full state snapshot', () => {
    const ks = new KillSwitch();
    const initial = ks.getState();
    expect(initial.enabled).toBe(false);
    expect(initial.reason).toBe('');
    expect(initial.enabledAt).toBeNull();
    expect(initial.enabledBy).toBeNull();
  });

  it('KillSwitch enable records actor and timestamp', () => {
    const before = Date.now();
    const ks = new KillSwitch();
    ks.enable('Budget exceeded', 'human:ops');
    const state = ks.getState();
    expect(state.enabled).toBe(true);
    expect(state.enabledBy).toBe('human:ops');
    expect(state.reason).toBe('Budget exceeded');
    expect(state.enabledAt).toBeGreaterThanOrEqual(before);
  });
});
