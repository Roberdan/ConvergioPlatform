export interface ManualTriggerResult {
  mode: 'daily' | 'weekly';
  cycleId: number;
}

export class ManualTriggerService {
  private readonly trigger: (mode: 'daily' | 'weekly') => Promise<ManualTriggerResult>;

  constructor(trigger: (mode: 'daily' | 'weekly') => Promise<ManualTriggerResult>) {
    this.trigger = trigger;
  }

  async manualTrigger(mode: 'daily' | 'weekly'): Promise<ManualTriggerResult> {
    return this.trigger(mode);
  }
}
