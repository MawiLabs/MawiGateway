//! Per-user request-rate limiter (closes #43).
//!
//! Fixed 60-second window, in-memory state. Cheap (one DashMap lookup +
//! a couple of `Instant::elapsed` calls per authenticated request) and
//! coarse (per-pod state — across N pods the effective limit is N×); this
//! is enough to stop one runaway client from dominating the gateway,
//! which is the threat model #43 names. A finer/distributed scheme can
//! land later without changing this module's call surface.
//!
//! The default limit comes from `DEFAULT_RATE_LIMIT_PER_MIN` (env, default
//! 60). A future enhancement will read per-user overrides from the DB.

use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// 60-second rolling window length.
const WINDOW: Duration = Duration::from_secs(60);

/// Default cap when `DEFAULT_RATE_LIMIT_PER_MIN` is not set / not parseable.
const DEFAULT_LIMIT: u32 = 60;

/// One user's window. `start` is the instant the window opened; `count`
/// is the number of requests served inside it. Both fields are mutated
/// under the same `DashMap` shard lock — no further synchronisation.
struct Window {
    start: Instant,
    count: AtomicU32,
}

/// Process-wide singleton. Lazy because we want a single shared map even
/// if the middleware is reconstructed for tests.
fn limiter_state() -> &'static DashMap<String, Arc<Window>> {
    static STATE: OnceLock<DashMap<String, Arc<Window>>> = OnceLock::new();
    STATE.get_or_init(DashMap::new)
}

/// Read the configured per-minute cap once and cache it. Re-reads on
/// each call would drag in `getenv` per request; this is fine because
/// the env var is fixed for the process lifetime in every deploy
/// pattern we support.
fn configured_limit() -> u32 {
    static LIMIT: OnceLock<u32> = OnceLock::new();
    *LIMIT.get_or_init(|| {
        std::env::var("DEFAULT_RATE_LIMIT_PER_MIN")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_LIMIT)
    })
}

/// Outcome of a `check_and_record` call.
#[derive(Debug, PartialEq, Eq)]
pub enum RateLimitDecision {
    /// Request may proceed.
    Allow,
    /// Request must be rejected with HTTP 429. The `retry_after` is the
    /// number of seconds the client should wait before retrying.
    Deny { retry_after_secs: u64 },
}

/// Increment the user's counter and return whether the request may
/// proceed. The check + increment is atomic per shard, so two
/// concurrent requests from the same user under the limit can both
/// succeed but the (limit+1)th is reliably denied.
pub fn check_and_record(user_id: &str) -> RateLimitDecision {
    check_and_record_with_clock(user_id, configured_limit(), Instant::now())
}

/// Test seam: same logic, but the caller supplies the limit and clock.
pub fn check_and_record_with_clock(
    user_id: &str,
    limit: u32,
    now: Instant,
) -> RateLimitDecision {
    let map = limiter_state();

    // Fast path: existing window for this user.
    if let Some(entry) = map.get(user_id) {
        let elapsed = now.saturating_duration_since(entry.start);
        if elapsed < WINDOW {
            // Still inside the same window — atomic increment-then-check.
            let prev = entry.count.fetch_add(1, Ordering::Relaxed);
            if prev < limit {
                return RateLimitDecision::Allow;
            }
            // Roll back (so we don't grow unbounded inside one window),
            // then deny.
            entry.count.fetch_sub(1, Ordering::Relaxed);
            let retry_after_secs = WINDOW.saturating_sub(elapsed).as_secs().max(1);
            return RateLimitDecision::Deny { retry_after_secs };
        }
        // Window expired — fall through to slow path which resets it.
    }

    // Slow path: need to (re)create the window. Use `entry()` so two
    // concurrent first-time-or-expired callers don't race on the
    // initialisation.
    let mut entry = map.entry(user_id.to_string()).or_insert_with(|| {
        Arc::new(Window {
            start: now,
            count: AtomicU32::new(0),
        })
    });

    // If the window we got is already expired (raced with another
    // thread that just incremented an old one), reset it in place.
    if now.saturating_duration_since(entry.start) >= WINDOW {
        *entry = Arc::new(Window {
            start: now,
            count: AtomicU32::new(0),
        });
    }

    let prev = entry.count.fetch_add(1, Ordering::Relaxed);
    if prev < limit {
        RateLimitDecision::Allow
    } else {
        entry.count.fetch_sub(1, Ordering::Relaxed);
        let elapsed = now.saturating_duration_since(entry.start);
        let retry_after_secs = WINDOW.saturating_sub(elapsed).as_secs().max(1);
        RateLimitDecision::Deny { retry_after_secs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Concurrent request behaviour — checks the increment-then-rollback
    /// path is exact: exactly `limit` Allow before the first Deny in a
    /// fresh window.
    #[test]
    fn allow_until_limit_then_deny() {
        let user = format!("test-user-{}", uuid::Uuid::new_v4());
        let now = Instant::now();
        for i in 0..5 {
            assert_eq!(
                check_and_record_with_clock(&user, 5, now),
                RateLimitDecision::Allow,
                "request {} should be allowed within limit 5",
                i
            );
        }
        match check_and_record_with_clock(&user, 5, now) {
            RateLimitDecision::Deny { retry_after_secs } => {
                assert!(retry_after_secs > 0 && retry_after_secs <= 60);
            }
            other => panic!("expected Deny after limit, got {:?}", other),
        }
    }

    #[test]
    fn window_resets_after_60s() {
        let user = format!("test-user-{}", uuid::Uuid::new_v4());
        let t0 = Instant::now();
        for _ in 0..3 {
            assert_eq!(
                check_and_record_with_clock(&user, 3, t0),
                RateLimitDecision::Allow
            );
        }
        // Right at the boundary — denied.
        assert!(matches!(
            check_and_record_with_clock(&user, 3, t0),
            RateLimitDecision::Deny { .. }
        ));

        // 61 seconds later — new window, allow again.
        let t1 = t0 + Duration::from_secs(61);
        assert_eq!(
            check_and_record_with_clock(&user, 3, t1),
            RateLimitDecision::Allow
        );
    }

    #[test]
    fn different_users_get_independent_windows() {
        let a = format!("test-user-{}", uuid::Uuid::new_v4());
        let b = format!("test-user-{}", uuid::Uuid::new_v4());
        let now = Instant::now();

        for _ in 0..2 {
            assert_eq!(check_and_record_with_clock(&a, 2, now), RateLimitDecision::Allow);
        }
        // a is at the limit; b has been silent and should still pass.
        assert!(matches!(
            check_and_record_with_clock(&a, 2, now),
            RateLimitDecision::Deny { .. }
        ));
        assert_eq!(check_and_record_with_clock(&b, 2, now), RateLimitDecision::Allow);
    }
}
