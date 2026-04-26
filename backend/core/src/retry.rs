//! Provider retry policy with exponential backoff + full jitter.
//!
//! Use case: transient upstream errors (429, 503, 504, network timeouts) get
//! retried up to a small number of attempts; permanent errors (400, 401)
//! return immediately. The backoff respects the upstream's `Retry-After`
//! hint when present.
//!
//! Pattern:
//!
//!   let policy = RetryPolicy::default();
//!   let res = retry::retry(policy, "openai.chat", || async {
//!       provider.stream_chat(req).await
//!   }).await;
//!
//! The closure is invoked once per attempt; failures get classified via
//! `error::downcast` and retried only if `ProviderError::is_retryable()`.

use std::future::Future;
use std::time::Duration;

use rand::Rng;

use crate::error;

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    /// Total attempts including the first call. `1` disables retries.
    pub max_attempts: u32,
    /// Initial backoff between attempts.
    pub initial_backoff: Duration,
    /// Cap so a long Retry-After never blocks the request thread forever.
    pub max_backoff: Duration,
    /// Multiplier applied to backoff on each retry.
    pub backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            // 1 original + 2 retries — keeps p99 latency in check while
            // covering the typical 429-burst case.
            max_attempts: 3,
            initial_backoff: Duration::from_millis(250),
            // Hard cap. Even if the provider says "Retry-After: 600", we
            // give up after 30s rather than block the user request.
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Compute the wait before attempt `n` (1-indexed). Honours an explicit
    /// `Retry-After` from the provider. Adds full jitter (0..base).
    pub fn delay(&self, attempt: u32, retry_after: Option<Duration>) -> Duration {
        if let Some(server_hint) = retry_after {
            // Server told us how long; cap and trust it.
            return server_hint.min(self.max_backoff);
        }
        let base = self
            .initial_backoff
            .as_secs_f64()
            .min(self.max_backoff.as_secs_f64())
            * self.backoff_multiplier.powi((attempt - 1) as i32);
        let bounded = base.min(self.max_backoff.as_secs_f64()).max(0.0);
        let jitter = rand::thread_rng().gen_range(0.0..=bounded);
        Duration::from_secs_f64(jitter)
    }
}

/// Run `op` under the retry policy. Returns the last result (success or final
/// failure). `op_label` shows up in tracing events to make retries debuggable.
pub async fn retry<T, Fut, F>(
    policy: RetryPolicy,
    op_label: &str,
    mut op: F,
) -> Result<T, anyhow::Error>
where
    Fut: Future<Output = Result<T, anyhow::Error>>,
    F: FnMut() -> Fut,
{
    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 1..=policy.max_attempts {
        match op().await {
            Ok(value) => {
                if attempt > 1 {
                    tracing::info!(op = op_label, attempt, "retry succeeded");
                }
                return Ok(value);
            }
            Err(err) => {
                let classified = error::downcast(&err);
                let is_retryable = classified.map(|e| e.is_retryable()).unwrap_or(false);
                let last = attempt == policy.max_attempts;

                if !is_retryable || last {
                    if last && is_retryable {
                        tracing::warn!(
                            op = op_label,
                            attempt,
                            error = %err,
                            "retry exhausted"
                        );
                    } else {
                        tracing::debug!(
                            op = op_label,
                            attempt,
                            error = %err,
                            retryable = is_retryable,
                            "no retry"
                        );
                    }
                    return Err(err);
                }

                let retry_after = classified.and_then(|e| e.retry_after());
                let delay = policy.delay(attempt + 1, retry_after);
                tracing::warn!(
                    op = op_label,
                    attempt,
                    error = %err,
                    delay_ms = delay.as_millis() as u64,
                    "retrying after transient failure"
                );
                last_err = Some(err);
                tokio::time::sleep(delay).await;
            }
        }
    }
    // Unreachable in practice — the loop returns inside.
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("retry policy exhausted with no error")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ProviderError;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn returns_first_success_immediately() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_for_op = calls.clone();
        let res = retry(RetryPolicy::default(), "test.ok", move || {
            let calls_clone = calls_for_op.clone();
            async move {
                calls_clone.fetch_add(1, Ordering::SeqCst);
                Ok::<u32, anyhow::Error>(42)
            }
        })
        .await
        .unwrap();
        assert_eq!(res, 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_on_retryable_error() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_for_op = calls.clone();
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(1),
            max_backoff: Duration::from_millis(5),
            backoff_multiplier: 2.0,
        };
        let res = retry(policy, "test.flaky", move || {
            let calls_clone = calls_for_op.clone();
            async move {
                let n = calls_clone.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(anyhow::Error::new(ProviderError::Unavailable {
                        provider: "test".into(),
                        message: "boom".into(),
                        retry_after: None,
                    }))
                } else {
                    Ok::<u32, anyhow::Error>(n)
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(res, 2);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn does_not_retry_non_retryable() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_for_op = calls.clone();
        let res: Result<(), anyhow::Error> = retry(
            RetryPolicy {
                max_attempts: 3,
                initial_backoff: Duration::from_millis(1),
                max_backoff: Duration::from_millis(5),
                backoff_multiplier: 2.0,
            },
            "test.bad",
            move || {
                let calls_clone = calls_for_op.clone();
                async move {
                    calls_clone.fetch_add(1, Ordering::SeqCst);
                    Err(anyhow::Error::new(ProviderError::BadRequest {
                        provider: "test".into(),
                        message: "you sent garbage".into(),
                    }))
                }
            },
        )
        .await;
        assert!(res.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn server_retry_after_is_capped() {
        let policy = RetryPolicy::default();
        let huge = Duration::from_secs(600);
        assert_eq!(policy.delay(2, Some(huge)), policy.max_backoff);
    }
}
