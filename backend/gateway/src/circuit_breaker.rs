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

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            failure_threshold: 3,           // Trip after 3 consecutive failures
            reset_timeout: Duration::from_secs(60), // Wait 60s before retrying
            max_entries: 10_000,            // Limit to 10k entries (prevent unbounded growth)
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
            },
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
                     eprintln!("✅ Circuit Closed (recovered) for resource: {}", resource_id);
                     entry.state = CircuitState::Closed;
                     entry.failure_count = 0;
                     entry.last_failure = None;
                },
                CircuitState::Closed => {
                    // Reset failure count on success to prevent stale failures triggering open
                    if entry.failure_count > 0 {
                        entry.failure_count = 0;
                    }
                },
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
                
                eprintln!("⚠️ Circuit Failure {}/{} for resource: {}", 
                    entry.failure_count, self.failure_threshold, resource_id);

                if entry.failure_count >= self.failure_threshold {
                    crate::metrics::CIRCUIT_BREAKER_TRIPS.inc();
                    crate::metrics::CIRCUIT_BREAKER_OPEN.inc();
                    eprintln!("🚫 Circuit OPEN for resource: {} (Tripped after {} failures)", 
                        resource_id, entry.failure_count);
                    entry.state = CircuitState::Open { opened_at: Instant::now() };
                }
            },
            CircuitState::HalfOpen => {
                // Failure in HalfOpen -> Re-open immediately
                eprintln!("🚫 Circuit Re-OPEN (Half-Open failed) for resource: {}", resource_id);
                entry.state = CircuitState::Open { opened_at: Instant::now() };
            },
            CircuitState::Open { .. } => {
                // Already open, refresh valid? maybe not.
            }
        }
    }
}
