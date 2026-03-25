//! Async retry with exponential backoff and optional jitter.
//!
//! Jitter randomizes the delay to prevent thundering-herd when many callers
//! retry at the same time after a service recovers.

use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

// rand 0.10 provides random_range as a free function

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Number of additional attempts after the initial one (0 = no retry).
    pub max_retries: u32,
    /// Delay before the first retry.
    pub initial_delay: Duration,
    /// Maximum delay between retries (caps exponential growth).
    pub max_delay: Duration,
    /// Multiplier applied to delay after each failure (e.g. 2.0 = double).
    pub backoff_factor: f64,
    /// When true, adds ±25% random jitter to each computed delay.
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_factor: 2.0,
            jitter: true,
        }
    }
}

/// Retries an async operation with exponential backoff.
///
/// `f` is a factory called each attempt; it returns a `Future<Output = Result<T, E>>`.
/// On success the value is returned immediately.
/// On failure the result of the final attempt is returned.
///
/// # Example
/// ```ignore
/// let result = retry_with_backoff(
///     || async { call_external_api().await },
///     RetryConfig::default(),
/// ).await;
/// ```
pub async fn retry_with_backoff<F, Fut, T, E>(mut f: F, config: RetryConfig) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let total_attempts = config.max_retries + 1;
    let mut delay = config.initial_delay;

    for attempt in 0..total_attempts {
        match f().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                if attempt + 1 >= total_attempts {
                    return Err(err);
                }

                let sleep_duration = apply_jitter(delay, config.jitter);
                sleep(sleep_duration).await;

                // Compute next delay with capped exponential backoff
                let next_ms = (delay.as_millis() as f64 * config.backoff_factor) as u128;
                delay = Duration::from_millis(next_ms.min(config.max_delay.as_millis()) as u64);
            }
        }
    }

    // Unreachable: loop returns on last attempt
    unreachable!("retry loop exhausted without returning")
}

/// Applies ±25% jitter when enabled.
fn apply_jitter(delay: Duration, enabled: bool) -> Duration {
    if !enabled {
        return delay;
    }
    let ms = delay.as_millis() as f64;
    let jitter_factor = rand::random_range(0.75_f64..1.25_f64);
    Duration::from_millis((ms * jitter_factor) as u64)
}

/// Synchronous retry with exponential backoff.
///
/// Used for blocking contexts such as SQLite operations.
/// `is_retryable` controls which errors trigger a retry; non-retryable errors
/// are returned immediately without consuming further attempts.
pub fn retry_sync<T, E, F, P>(
    mut f: F,
    config: RetryConfig,
    is_retryable: P,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    P: Fn(&E) -> bool,
{
    let total_attempts = config.max_retries + 1;
    let mut delay = config.initial_delay;

    for attempt in 0..total_attempts {
        match f() {
            Ok(value) => return Ok(value),
            Err(err) => {
                if attempt + 1 >= total_attempts || !is_retryable(&err) {
                    return Err(err);
                }
                let sleep_duration = apply_jitter(delay, config.jitter);
                std::thread::sleep(sleep_duration);

                let next_ms = (delay.as_millis() as f64 * config.backoff_factor) as u128;
                delay = Duration::from_millis(next_ms.min(config.max_delay.as_millis()) as u64);
            }
        }
    }
    unreachable!("sync retry loop exhausted without returning")
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn single_attempt_no_delay_on_success() {
        let count = Arc::new(Mutex::new(0u32));
        let c = Arc::clone(&count);
        let result = retry_with_backoff(
            || {
                *c.lock().unwrap() += 1;
                async { Ok::<_, String>(1) }
            },
            RetryConfig {
                max_retries: 5,
                initial_delay: Duration::from_millis(1),
                max_delay: Duration::from_millis(10),
                backoff_factor: 2.0,
                jitter: false,
            },
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn zero_retries_means_one_attempt() {
        let count = Arc::new(Mutex::new(0u32));
        let c = Arc::clone(&count);
        let result: Result<(), String> = retry_with_backoff(
            || {
                *c.lock().unwrap() += 1;
                async { Err("fail".to_string()) }
            },
            RetryConfig {
                max_retries: 0,
                initial_delay: Duration::from_millis(1),
                max_delay: Duration::from_millis(1),
                backoff_factor: 1.0,
                jitter: false,
            },
        )
        .await;
        assert!(result.is_err());
        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[test]
    fn jitter_does_not_exceed_double_delay() {
        // Run many samples; none should be more than 1.25× the input
        let base = Duration::from_millis(100);
        for _ in 0..100 {
            let jittered = apply_jitter(base, true);
            assert!(
                jittered.as_millis() <= 125,
                "jitter exceeded 1.25×: {jittered:?}"
            );
            assert!(
                jittered.as_millis() >= 75,
                "jitter below 0.75×: {jittered:?}"
            );
        }
    }
}
