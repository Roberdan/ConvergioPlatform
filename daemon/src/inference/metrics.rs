// Inference metrics collection — rolling windows with per-model stats.
// Tracks latency percentiles, error rate, cost, and throughput per model.
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

// ---- public types --------------------------------------------------------

/// Single observation recorded after an inference call.
#[derive(Debug, Clone)]
pub struct InferenceMetricsEntry {
    pub model: String,
    pub latency_ms: u64,
    pub tokens_used: u32,
    pub cost: f64,
    pub success: bool,
    pub timestamp: DateTime<Utc>,
}

/// Time window for metric queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeWindow {
    OneHour,
    TwentyFourHours,
    SevenDays,
}

impl TimeWindow {
    /// Duration represented by this window.
    fn duration(self) -> Duration {
        match self {
            TimeWindow::OneHour => Duration::hours(1),
            TimeWindow::TwentyFourHours => Duration::hours(24),
            TimeWindow::SevenDays => Duration::days(7),
        }
    }

    /// Short label used in serialized output.
    pub fn label(self) -> &'static str {
        match self {
            TimeWindow::OneHour => "1h",
            TimeWindow::TwentyFourHours => "24h",
            TimeWindow::SevenDays => "7d",
        }
    }
}

/// Computed statistics for one model over a specific time window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetrics {
    pub model: String,
    pub request_count: usize,
    pub error_rate: f64,
    pub latency_p50: u64,
    pub latency_p95: u64,
    pub latency_p99: u64,
    pub avg_tokens_per_sec: f64,
    pub avg_cost: f64,
    pub window_label: String,
}

// ---- collector -----------------------------------------------------------

/// Bounded in-memory store of inference observations; supports time-windowed queries.
///
/// Entries older than `SevenDays` are pruned on each `record` call to
/// prevent unbounded growth while keeping the full 7-day history.
pub struct InferenceMetricsCollector {
    entries: Vec<InferenceMetricsEntry>,
}

impl InferenceMetricsCollector {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Append one observation and evict entries beyond the maximum retention window.
    pub fn record(&mut self, entry: InferenceMetricsEntry) {
        self.entries.push(entry);
        let cutoff = Utc::now() - TimeWindow::SevenDays.duration();
        self.entries.retain(|e| e.timestamp >= cutoff);
    }

    /// Compute stats for `model` within `window`. Returns zero-valued metrics
    /// when the model has no observations, so callers can always render safely.
    pub fn metrics_for(&self, model: &str, window: TimeWindow) -> ModelMetrics {
        let cutoff = Utc::now() - window.duration();
        let relevant: Vec<&InferenceMetricsEntry> = self
            .entries
            .iter()
            .filter(|e| e.model == model && e.timestamp >= cutoff)
            .collect();

        compute_metrics(model.to_string(), &relevant, window)
    }

    /// Compute stats for every known model within `window`.
    pub fn all_metrics(&self, window: TimeWindow) -> Vec<ModelMetrics> {
        let cutoff = Utc::now() - window.duration();

        // Collect unique model names present in the window.
        let mut models: Vec<String> = self
            .entries
            .iter()
            .filter(|e| e.timestamp >= cutoff)
            .map(|e| e.model.clone())
            .collect();
        models.sort();
        models.dedup();

        models
            .into_iter()
            .map(|m| self.metrics_for(&m, window))
            .collect()
    }
}

impl Default for InferenceMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ---- internal computation ------------------------------------------------

fn compute_metrics(
    model: String,
    entries: &[&InferenceMetricsEntry],
    window: TimeWindow,
) -> ModelMetrics {
    if entries.is_empty() {
        return ModelMetrics {
            model,
            request_count: 0,
            error_rate: 0.0,
            latency_p50: 0,
            latency_p95: 0,
            latency_p99: 0,
            avg_tokens_per_sec: 0.0,
            avg_cost: 0.0,
            window_label: window.label().to_string(),
        };
    }

    let request_count = entries.len();
    let error_count = entries.iter().filter(|e| !e.success).count();
    let error_rate = error_count as f64 / request_count as f64;

    // Latency percentiles over ALL entries (success + failure) for honest SLA.
    let mut latencies: Vec<u64> = entries.iter().map(|e| e.latency_ms).collect();
    latencies.sort_unstable();
    let latency_p50 = percentile(&latencies, 50);
    let latency_p95 = percentile(&latencies, 95);
    let latency_p99 = percentile(&latencies, 99);

    // Throughput: tokens per second averaged over successful requests only.
    let successful: Vec<&&InferenceMetricsEntry> =
        entries.iter().filter(|e| e.success).collect();
    let avg_tokens_per_sec = if successful.is_empty() {
        0.0
    } else {
        let total_tps: f64 = successful
            .iter()
            .map(|e| {
                if e.latency_ms == 0 {
                    0.0
                } else {
                    e.tokens_used as f64 / (e.latency_ms as f64 / 1000.0)
                }
            })
            .sum();
        total_tps / successful.len() as f64
    };

    let avg_cost = entries.iter().map(|e| e.cost).sum::<f64>() / request_count as f64;

    ModelMetrics {
        model,
        request_count,
        error_rate,
        latency_p50,
        latency_p95,
        latency_p99,
        avg_tokens_per_sec,
        avg_cost,
        window_label: window.label().to_string(),
    }
}

/// Nearest-rank percentile (1-indexed, rounds up).
fn percentile(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((p as f64 / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[idx.saturating_sub(1).min(sorted.len() - 1)]
}

// ---- tests ---------------------------------------------------------------

#[cfg(test)]
#[path = "metrics_tests.rs"]
mod tests;
