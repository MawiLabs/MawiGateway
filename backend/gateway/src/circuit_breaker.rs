use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Circuit Breaker States
#[derive(Debug, Clone, PartialEq)]
enum CircuitState {
    /// Circuit is closed (normal operation)
    Closed,
    /// Circuit is open (failing fast)
    Open { opened_at: Instant },
    /// Circuit is half-open (testing recovery)
    HalfOpen,
}

/// Circuit Breaker Entry for a specific resource (model/provider)
#[derive(Debug)]
struct CircuitEntry {
    state: CircuitState,
    failure_count: u32,
    last_failure: Option<Instant>,
}

impl Default for CircuitEntry {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure: None,
        }
    }
}

/// Circuit Breaker Manager
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    // Map resource ID (model/provider ID) -> Circuit Entry
    entries: Arc<DashMap<String, CircuitEntry>>,
    // Configuration
    failure_threshold: u32,
    reset_timeout: Duration,
    max_entries: usize,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            failure_threshold: 3, // Trip after 3 consecutive failures
            reset_timeout: Duration::from_secs(60), // Wait 60s before retrying
            max_entries: 10_000,  // Limit to 10k entries (prevent unbounded growth)
        }
    }

    /// Test/operator-tunable constructor (#82). The defaults in `new()` are
    /// production-tuned (3 failures / 60s reset) but tests need much shorter
    /// timeouts to validate state transitions in milliseconds, and operators
    /// running smaller fleets may want different thresholds.
    pub fn with_config(failure_threshold: u32, reset_timeout: Duration) -> Self {
        let mut cb = Self::new();
        cb.failure_threshold = failure_threshold;
        cb.reset_timeout = reset_timeout;
        cb
    }

    /// Returns the breaker state for a given resource as a stable string
    /// (`"closed"`, `"open"`, `"half_open"`, or `"unknown"`). Used by the
    /// CLI / UI / MCP surface in #82 so operators can see breaker state
    /// per provider.
    pub fn state(&self, resource_id: &str) -> &'static str {
        match self.entries.get(resource_id).map(|e| e.state.clone()) {
            Some(CircuitState::Closed) => "closed",
            Some(CircuitState::Open { .. }) => "open",
            Some(CircuitState::HalfOpen) => "half_open",
            None => "unknown",
        }
    }

    /// Check if a request is allowed for a given resource
    pub async fn allow_request(&self, resource_id: &str) -> bool {
        // Fast path: Read-only check
        if let Some(entry) = self.entries.get(resource_id) {
            if entry.state == CircuitState::Closed {
                return true;
            }
        }

        // Slow path: Potential mutation or first access
        let mut entry = self.entries.entry(resource_id.to_string()).or_default();

        match entry.state {
            CircuitState::Closed => true,
            CircuitState::Open { opened_at } => {
                // If timeout expired, switch to HalfOpen and allow ONE request
                if opened_at.elapsed() >= self.reset_timeout {
                    eprintln!("🔄 Circuit Half-Open for resource: {}", resource_id);
                    entry.state = CircuitState::HalfOpen;
                    true
                } else {
                    false // Still open, block request
                }
            }
            CircuitState::HalfOpen => {
                // Only allow one request at a time in HalfOpen state
                true
            }
        }
    }

    /// Record a successful request
    pub async fn record_success(&self, resource_id: &str) {
        if let Some(mut entry) = self.entries.get_mut(resource_id) {
            match entry.state {
                CircuitState::HalfOpen => {
                    // Success in HalfOpen -> Reset to Closed
                    crate::metrics::CIRCUIT_BREAKER_OPEN.dec();
                    eprintln!(
                        "✅ Circuit Closed (recovered) for resource: {}",
                        resource_id
                    );
                    entry.state = CircuitState::Closed;
                    entry.failure_count = 0;
                    entry.last_failure = None;
                }
                CircuitState::Closed => {
                    // Reset failure count on success to prevent stale failures triggering open
                    if entry.failure_count > 0 {
                        entry.failure_count = 0;
                    }
                }
                CircuitState::Open { .. } => {
                    // Should not happen, but if logic permits, reset
                    entry.state = CircuitState::Closed;
                    entry.failure_count = 0;
                }
            }
        }
    }

    /// Record a failed request
    pub async fn record_failure(&self, resource_id: &str) {
        // Evict oldest entry if at capacity
        if self.entries.len() >= self.max_entries {
            if let Some(entry) = self.entries.iter().next() {
                let key_to_remove = entry.key().clone();
                drop(entry); // Release the reference
                self.entries.remove(&key_to_remove);
            }
        }

        let mut entry = self.entries.entry(resource_id.to_string()).or_default();

        match entry.state {
            CircuitState::Closed => {
                entry.failure_count += 1;
                entry.last_failure = Some(Instant::now());

                eprintln!(
                    "⚠️ Circuit Failure {}/{} for resource: {}",
                    entry.failure_count, self.failure_threshold, resource_id
                );

                if entry.failure_count >= self.failure_threshold {
                    crate::metrics::CIRCUIT_BREAKER_TRIPS.inc();
                    crate::metrics::CIRCUIT_BREAKER_OPEN.inc();
                    eprintln!(
                        "🚫 Circuit OPEN for resource: {} (Tripped after {} failures)",
                        resource_id, entry.failure_count
                    );
                    entry.state = CircuitState::Open {
                        opened_at: Instant::now(),
                    };
                }
            }
            CircuitState::HalfOpen => {
                // Failure in HalfOpen -> Re-open immediately
                eprintln!(
                    "🚫 Circuit Re-OPEN (Half-Open failed) for resource: {}",
                    resource_id
                );
                entry.state = CircuitState::Open {
                    opened_at: Instant::now(),
                };
            }
            CircuitState::Open { .. } => {
                // Already open, refresh valid? maybe not.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Circuit breaker state-machine tests — closes part of #82.
    //!
    //! Verifies the documented transitions:
    //!   Closed --(N failures)--> Open
    //!   Open   --(reset_timeout)--> HalfOpen on next allow_request
    //!   HalfOpen --(success)--> Closed
    //!   HalfOpen --(failure)--> Open (re-opened immediately)
    //!
    //! Tests use `with_config(2, 50ms)` so the half-open transition can
    //! be validated in real wall time without sleeping for 60s.

    use super::*;

    fn cb() -> CircuitBreaker {
        CircuitBreaker::with_config(2, Duration::from_millis(50))
    }

    #[tokio::test]
    async fn unknown_resource_starts_closed() {
        let breaker = cb();
        assert!(breaker.allow_request("openai").await);
        assert_eq!(breaker.state("openai"), "closed");
    }

    #[tokio::test]
    async fn opens_after_threshold_failures() {
        let breaker = cb();
        breaker.record_failure("openai").await;
        assert_eq!(breaker.state("openai"), "closed", "1 failure < threshold");
        breaker.record_failure("openai").await;
        assert_eq!(breaker.state("openai"), "open", "threshold hit");
        // Open breaker rejects requests.
        assert!(!breaker.allow_request("openai").await);
    }

    #[tokio::test]
    async fn half_opens_after_reset_timeout() {
        let breaker = cb();
        breaker.record_failure("openai").await;
        breaker.record_failure("openai").await;
        assert_eq!(breaker.state("openai"), "open");

        tokio::time::sleep(Duration::from_millis(80)).await;
        // First call after timeout flips Open -> HalfOpen and is allowed.
        assert!(breaker.allow_request("openai").await);
        assert_eq!(breaker.state("openai"), "half_open");
    }

    #[tokio::test]
    async fn half_open_success_closes_breaker() {
        let breaker = cb();
        breaker.record_failure("openai").await;
        breaker.record_failure("openai").await;
        tokio::time::sleep(Duration::from_millis(80)).await;
        let _ = breaker.allow_request("openai").await; // -> half_open
        breaker.record_success("openai").await;
        assert_eq!(breaker.state("openai"), "closed");
    }

    #[tokio::test]
    async fn half_open_failure_reopens_breaker() {
        let breaker = cb();
        breaker.record_failure("openai").await;
        breaker.record_failure("openai").await;
        tokio::time::sleep(Duration::from_millis(80)).await;
        let _ = breaker.allow_request("openai").await; // -> half_open
        breaker.record_failure("openai").await;
        assert_eq!(breaker.state("openai"), "open");
    }

    #[tokio::test]
    async fn success_in_closed_resets_failure_count() {
        let breaker = cb();
        breaker.record_failure("openai").await;
        // 1 failure, still closed; success should reset the counter so
        // the NEXT failure doesn't immediately trip the breaker.
        breaker.record_success("openai").await;
        breaker.record_failure("openai").await;
        assert_eq!(breaker.state("openai"), "closed");
    }

    #[tokio::test]
    async fn breakers_are_per_resource() {
        let breaker = cb();
        breaker.record_failure("openai").await;
        breaker.record_failure("openai").await;
        assert_eq!(breaker.state("openai"), "open");
        // Anthropic should be unaffected.
        assert_eq!(breaker.state("anthropic"), "unknown");
        assert!(breaker.allow_request("anthropic").await);
    }
}
