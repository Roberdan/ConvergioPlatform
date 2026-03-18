import { describe, expect, it, vi } from 'vitest';
import { CronConfig } from '../../../evolution/cadence/cron-config.ts';
import { ManualTriggerService } from '../../../evolution/cadence/manual-trigger.ts';

describe('CronConfig', () => {
  it('parses cronExpression and matches schedule fields', () => {
    const cfg = new CronConfig();
    const parsed = cfg.parse('5 10 * * *');
    expect(cfg.matches(parsed, new Date('2026-03-18T10:05:00.000Z'))).toBe(true);
    expect(cfg.matches(parsed, new Date('2026-03-18T10:06:00.000Z'))).toBe(false);
  });
});

describe('ManualTriggerService', () => {
  it('delegates manualTrigger calls', async () => {
    const triggerManual = vi.fn(async (mode: 'daily' | 'weekly') => ({ mode, cycleId: 1 }));
    const service = new ManualTriggerService(triggerManual);

    const daily = await service.manualTrigger('daily');
    const weekly = await service.manualTrigger('weekly');

    expect(triggerManual).toHaveBeenCalledTimes(2);
    expect(daily.mode).toBe('daily');
    expect(weekly.mode).toBe('weekly');
  });
});
