import type { MetricCollector } from '../core/types/index.js';

export class CollectorRegistry {
  private readonly byId = new Map<string, MetricCollector>();

  register(collector: MetricCollector): void {
    this.byId.set(collector.id, collector);
  }

  unregister(collectorId: string): void {
    this.byId.delete(collectorId);
  }

  list(): MetricCollector[] {
    return [...this.byId.values()];
  }

  ids(): string[] {
    return [...this.byId.keys()];
  }

  size(): number {
    return this.byId.size;
  }
}
