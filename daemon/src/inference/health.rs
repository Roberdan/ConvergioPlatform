// Health check system for inference endpoints.
// Tracks latency, error rate, and consecutive failures to classify endpoint health.
use std::collections::VecDeque;
use std::time::Duration;

/// Maximum latency samples retained per endpoint (ring buffer).
const RING_CAPACITY: usize = 20;
/// Latency threshold above which an endpoint is considered degraded.
const DEGRADED_LATENCY_MS: u128 = 2_000;
/// Error-rate threshold (0.0–1.0) above which an endpoint is considered degraded.
const DEGRADED_ERROR_RATE: f64 = 0.05;
/// Number of consecutive failures that marks an endpoint as down.
const DOWN_CONSECUTIVE_FAILURES: u32 = 3;

/// Result of a single availability probe.
#[derive(Debug, Clone)]
pub enum ProbeResult {
    Success(Duration),
    Error(String),
}

/// Health classification for an endpoint.
#[derive(Debug, PartialEq, Clone)]
pub enum EndpointHealthStatus {
    Healthy,
    /// Endpoint is reachable but performing below threshold; reason explains why.
    Degraded(String),
    Down,
}

/// Per-endpoint health state.
#[derive(Debug, Clone)]
pub struct EndpointHealth {
    pub name: String,
    /// Ring buffer of recent successful latencies (bounded to RING_CAPACITY).
    latencies: VecDeque<Duration>,
    /// Total probes recorded since last reset.
    total_probes: u32,
    /// Number of probes that were errors.
    error_count: u32,
    /// How many consecutive probes have been errors.
    consecutive_failures: u32,
    /// Timestamp of last probe (seconds since UNIX epoch, approximate).
    pub last_check_ms: Option<u64>,
}

impl EndpointHealth {
    /// Create a new, empty health record for `name`.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            latencies: VecDeque::with_capacity(RING_CAPACITY),
            total_probes: 0,
            error_count: 0,
            consecutive_failures: 0,
            last_check_ms: None,
        }
    }

    /// Record a probe result, maintaining the ring buffer.
    pub fn record_probe(&mut self, result: ProbeResult) {
        self.total_probes += 1;
        match result {
            ProbeResult::Success(latency) => {
                self.consecutive_failures = 0;
                if self.latencies.len() == RING_CAPACITY {
                    self.latencies.pop_front();
                }
                self.latencies.push_back(latency);
            }
            ProbeResult::Error(_) => {
                self.consecutive_failures += 1;
                self.error_count += 1;
            }
        }
    }

    /// Average latency in milliseconds over the sample ring buffer (0 if no data).
    pub fn avg_latency_ms(&self) -> u64 {
        if self.latencies.is_empty() {
            return 0;
        }
        let total: u128 = self.latencies.iter().map(|d| d.as_millis()).sum();
        (total / self.latencies.len() as u128) as u64
    }

    /// Compute current health status from recorded probes.
    pub fn status(&self) -> EndpointHealthStatus {
        // Down takes priority: 3+ consecutive failures.
        if self.consecutive_failures >= DOWN_CONSECUTIVE_FAILURES {
            return EndpointHealthStatus::Down;
        }

        // Check error rate first when we have enough data.
        if self.total_probes > 0 {
            let error_rate = self.error_count as f64 / self.total_probes as f64;
            if error_rate > DEGRADED_ERROR_RATE {
                return EndpointHealthStatus::Degraded(format!(
                    "error rate {:.1}% exceeds threshold {:.0}%",
                    error_rate * 100.0,
                    DEGRADED_ERROR_RATE * 100.0,
                ));
            }
        }

        // Check average latency when we have latency samples.
        if !self.latencies.is_empty() {
            let total_ms: u128 = self.latencies.iter().map(|d| d.as_millis()).sum();
            let avg_ms = total_ms / self.latencies.len() as u128;
            if avg_ms > DEGRADED_LATENCY_MS {
                return EndpointHealthStatus::Degraded(format!(
                    "avg latency {}ms exceeds threshold {}ms",
                    avg_ms, DEGRADED_LATENCY_MS,
                ));
            }
        }

        EndpointHealthStatus::Healthy
    }
}

/// Manages health state for a collection of inference endpoints.
pub struct HealthChecker {
    endpoints: Vec<EndpointHealth>,
}

impl HealthChecker {
    /// Create a checker with pre-configured endpoints (by name, empty state).
    pub fn new(names: Vec<&str>) -> Self {
        Self {
            endpoints: names.iter().map(|n| EndpointHealth::new(n)).collect(),
        }
    }

    /// Create a checker from existing `EndpointHealth` records (used in tests).
    pub fn from_endpoints(endpoints: Vec<EndpointHealth>) -> Self {
        Self { endpoints }
    }

    /// Return the health status for `endpoint_name`.
    /// Unknown endpoints default to Healthy (no data = no known problems).
    pub fn status(&self, endpoint_name: &str) -> EndpointHealthStatus {
        self.endpoints
            .iter()
            .find(|ep| ep.name == endpoint_name)
            .map(|ep| ep.status())
            .unwrap_or(EndpointHealthStatus::Healthy)
    }

    /// Return average latency (ms) for `endpoint_name`. 0 if unknown or no data.
    pub fn latency_ms(&self, endpoint_name: &str) -> u64 {
        self.endpoints
            .iter()
            .find(|ep| ep.name == endpoint_name)
            .map(|ep| ep.avg_latency_ms())
            .unwrap_or(0)
    }

    /// Return the names of all tracked endpoints.
    pub fn endpoint_names(&self) -> Vec<String> {
        self.endpoints.iter().map(|ep| ep.name.clone()).collect()
    }

    /// Run a single probe against an HTTP endpoint and record the result.
    /// Returns the measured `ProbeResult` for the caller to act on.
    pub async fn probe_http(
        &mut self,
        name: &str,
        url: &str,
    ) -> ProbeResult {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        let start = std::time::Instant::now();
        let result = match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                ProbeResult::Success(start.elapsed())
            }
            Ok(resp) => ProbeResult::Error(format!("HTTP {}", resp.status())),
            Err(e) => ProbeResult::Error(e.to_string()),
        };

        if let Some(ep) = self.endpoints.iter_mut().find(|ep| ep.name == name) {
            ep.record_probe(result.clone());
        }

        result
    }
}

#[cfg(test)]
#[path = "health_tests.rs"]
mod tests;
