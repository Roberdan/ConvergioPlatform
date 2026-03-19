import { EvolutionEngine } from '../evolution/core/engine.js';
import { DailyRunner } from '../evolution/cadence/daily-runner.js';

async function main(): Promise<void> {
  const engine = new EvolutionEngine({ adapters: [] });
  const runner = new DailyRunner();
  const summary = await runner.run(engine);
  process.stdout.write(`${JSON.stringify(summary)}\n`);
}

void main();
