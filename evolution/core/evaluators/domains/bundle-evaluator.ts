import type { AggregatedPoint } from '../../../telemetry/aggregation.js';
import type { Metric, EvaluationResult } from '../../types/index.js';
import { BaseEvaluator } from '../base-evaluator.js';
import { latestValue, opportunities } from './shared.js';

const KB = 1024;
const MB = 1024 * 1024;

export class BundleEvaluator extends BaseEvaluator {
  readonly domain = 'bundle';
  readonly metricFamilies = ['Build', 'Bundle'] as const;

  protected async analyze(metrics: Metric[], _history: AggregatedPoint[]): Promise<Partial<EvaluationResult>> {
    const anomalies: EvaluationResult['anomalies'] = [];
    const jsSize = latestValue(metrics, 'bundle.js_size_bytes');
    const cssSize = latestValue(metrics, 'bundle.css_size_bytes');
    const buildDuration = latestValue(metrics, 'build.duration_ms');

    if (jsSize !== null && jsSize > MB) {
      anomalies.push({ metric: 'bundle.js_size_bytes', severity: 'high', detail: `JS bundle=${(jsSize / KB).toFixed(0)}KB > 1024KB` });
    } else if (jsSize !== null && jsSize > 500 * KB) {
      anomalies.push({ metric: 'bundle.js_size_bytes', severity: 'medium', detail: `JS bundle=${(jsSize / KB).toFixed(0)}KB > 500KB` });
    }

    if (cssSize !== null && cssSize > 100 * KB) {
      anomalies.push({ metric: 'bundle.css_size_bytes', severity: 'medium', detail: `CSS bundle=${(cssSize / KB).toFixed(0)}KB > 100KB` });
    }

    if (buildDuration !== null && buildDuration > 120_000) {
      anomalies.push({ metric: 'build.duration_ms', severity: 'medium', detail: `Build duration=${Math.round(buildDuration / 1000)}s > 120s` });
    }

    return {
      anomalies,
      opportunities: anomalies.length
        ? opportunities(this.domain, ['Tree-shake unused imports', 'Enable code splitting', 'Lazy-load routes'], '-15% to -40% bundle cost')
        : [],
    };
  }
}
