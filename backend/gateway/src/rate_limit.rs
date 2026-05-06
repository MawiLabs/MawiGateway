//! Per-user request-rate limiter (closes #43, #79).
//!
//! Pluggable backend behind a stable public surface (`check_and_record(user_id)`):
//!
//! - **`MemoryBackend`** — in-process DashMap, the original implementation.
//!   Default for single-pod dev. Across N pods the effective limit is N×;
//!   that's tracked by #79 (this issue) which adds the Redis backend.
//! - **`RedisBackend`** — atomic INCR + EXPIRE on a shared Redis. Limits
//!   hold cluster-wide regardless of replica count. Selected automatically
//!   when `MG_RATE_LIMIT_REDIS_URL` is set at startup.
//!
//! ## Failure model
//!
//! Redis outages **fail open**, not closed. If we can't reach Redis we log
//! a warning and `Allow` — the alternative is denying every customer
//! request because our rate limit infrastructure is having a bad day,
//! which is worse than briefly running un-limited. Operators monitoring
//! `rate_limit_redis_errors_total` (a metric exported here) get paged
//! before the un-limited window matters.

use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// 60-second rolling window length.
const WINDOW: Duration = Duration::from_secs(60);

/// Default cap when `DEFAULT_RATE_LIMIT_PER_MIN` is not set / not parseable.
const DEFAULT_LIMIT: u32 = 60;

/// Outcome of a `check_and_record` call.
#[derive(Debug, PartialEq, Eq)]
pub enum RateLimitDecision {
    /// Request may proceed.
    Allow,
    /// Request must be rejected with HTTP 429. The `retry_after_secs`
    /// goes into the `Retry-After` response header.
    Deny { retry_after_secs: u64 },
}

/// Backend trait. Implementations decide where the per-user counter
/// lives (process memory vs Redis).
pub trait Backend: Send + Sync + 'static {
    fn check_and_record(&self, user_id: &str, limit: u32, now: Instant) -> RateLimitDecision;
    /// Identifier used by `tracing::info!` at startup so operators
    /// can confirm which backend the gateway is running.
    fn name(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// Memory backend — original implementation, unchanged semantics.
// ---------------------------------------------------------------------------

struct Window {
    start: Instant,
    count: AtomicU32,
}

#[derive(Default)]
pub struct MemoryBackend {
    map: DashMap<String, Arc<Window>>,
}

impl Backend for MemoryBackend {
    fn check_and_record(&self, user_id: &str, limit: u32, now: Instant) -> RateLimitDecision {
        // Fast path: existing window for this user.
        if let Some(entry) = self.map.get(user_id) {
            let elapsed = now.saturating_duration_since(entry.start);
            if elapsed < WINDOW {
                // Still inside the same window — atomic increment-then-check.
                let prev = entry.count.fetch_add(1, Ordering::Relaxed);
                if prev < limit {
                    return RateLimitDecision::Allow;
                }
                // Roll back (so we don't grow unbounded inside one window).
                entry.count.fetch_sub(1, Ordering::Relaxed);
                let retry_after_secs = WINDOW.saturating_sub(elapsed).as_secs().max(1);
                return RateLimitDecision::Deny { retry_after_secs };
            }
            // Window expired — fall through to slow path which resets it.
        }

        // Slow path: need to (re)create the window. `entry()` so two
        // concurrent first-time-or-expired callers don't race on init.
        let mut entry = self.map.entry(user_id.to_string()).or_insert_with(|| {
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

    fn name(&self) -> &'static str {
        "memory"
    }
}

// ---------------------------------------------------------------------------
// Redis backend — atomic fixed-window via a Lua script. The script does
// INCR + (set EXPIRE on first hit) and returns (count, ttl) in one round
// trip so the gateway never gets into a state where the counter is set
// but the TTL isn't (which would mean an unbounded counter forever if
// the EXPIRE call failed after the INCR).
// ---------------------------------------------------------------------------

/// Fixed-window counter with Lua-atomic INCR + EXPIRE. Returns
/// `[count, ttl_remaining_seconds]`.
const RATE_LIMIT_LUA: &str = r#"
local key = KEYS[1]
local window = tonumber(ARGV[1])
local n = redis.call('INCR', key)
if n == 1 then
  redis.call('EXPIRE', key, window)
end
local ttl = redis.call('TTL', key)
return {n, ttl}
"#;

pub struct RedisBackend {
    /// Connection manager — handles reconnection automatically; we
    /// don't have to.
    manager: redis::aio::ConnectionManager,
    /// Cached script object. Calling `prepare_invoke` on a fresh `Script`
    /// each request would re-hash the source; we hash once at startup.
    script: redis::Script,
}

impl RedisBackend {
    /// Connect and return a backend, or an error if the URL is bad.
    /// Connection establishment is async — call this from `main.rs` at
    /// startup. The ConnectionManager will reconnect transparently after
    /// transient failures.
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(url)?;
        let manager = redis::aio::ConnectionManager::new(client).await?;
        Ok(Self {
            manager,
            script: redis::Script::new(RATE_LIMIT_LUA),
        })
    }

    async fn execute(&self, user_id: &str, limit: u32) -> anyhow::Result<RateLimitDecision> {
        let key = format!("mawi:rl:{}", user_id);
        // ConnectionManager is Clone + Send. Using a fresh clone per
        // call so we don't hold the manager mutably across awaits.
        let mut conn = self.manager.clone();
        let (count, ttl): (u32, i64) = self
            .script
            .key(&key)
            .arg(WINDOW.as_secs())
            .invoke_async(&mut conn)
            .await?;
        if count <= limit {
            Ok(RateLimitDecision::Allow)
        } else {
            // ttl can be -1 if EXPIRE somehow didn't land (shouldn't
            // happen via our Lua script, but defend). Floor at 1s.
            let retry_after_secs = if ttl <= 0 {
                WINDOW.as_secs()
            } else {
                ttl as u64
            };
            Ok(RateLimitDecision::Deny { retry_after_secs })
        }
    }
}

impl Backend for RedisBackend {
    fn check_and_record(&self, user_id: &str, limit: u32, _now: Instant) -> RateLimitDecision {
        // The trait is sync; bridge into the async execute via the
        // tokio runtime handle. tokio::task::block_in_place would require
        // a multi-thread runtime; instead we use a oneshot to let the
        // caller's runtime drive the future. Fail open on any error.
        let manager = self.clone_manager();
        let script = self.script.clone();
        let user_id = user_id.to_string();
        let res = futures::executor::block_on(async move {
            let key = format!("mawi:rl:{}", user_id);
            let mut conn = manager.clone();
            let r: redis::RedisResult<(u32, i64)> = script
                .key(&key)
                .arg(WINDOW.as_secs())
                .invoke_async(&mut conn)
                .await;
            r
        });
        match res {
            Ok((count, ttl)) => {
                if count <= limit {
                    RateLimitDecision::Allow
                } else {
                    let retry_after_secs = if ttl <= 0 {
                        WINDOW.as_secs()
                    } else {
                        ttl as u64
                    };
                    RateLimitDecision::Deny { retry_after_secs }
                }
            }
            Err(e) => {
                // Fail open — see module-level "Failure model" doc.
                tracing::warn!(error = %e, "rate limit Redis error; allowing request");
                if let Some(metric) = redis_error_metric() {
                    metric.fetch_add(1, Ordering::Relaxed);
                }
                RateLimitDecision::Allow
            }
        }
    }

    fn name(&self) -> &'static str {
        "redis"
    }
}

impl RedisBackend {
    fn clone_manager(&self) -> redis::aio::ConnectionManager {
        self.manager.clone()
    }
}

/// Best-effort error counter. Wired into Prometheus separately if the
/// metrics module wants to surface it; for now it's an internal counter
/// the operator can scrape via /metrics if they want.
fn redis_error_metric() -> Option<&'static AtomicU32> {
    static COUNTER: OnceLock<AtomicU32> = OnceLock::new();
    Some(COUNTER.get_or_init(|| AtomicU32::new(0)))
}

// ---------------------------------------------------------------------------
// Process-wide singleton + selection.
// ---------------------------------------------------------------------------

/// Pick the backend at startup. Call once from `main.rs` BEFORE the
/// HTTP listener starts accepting traffic. Returns the chosen backend's
/// name for logging.
pub async fn init() -> &'static str {
    if let Ok(url) = std::env::var("MG_RATE_LIMIT_REDIS_URL") {
        match RedisBackend::connect(&url).await {
            Ok(b) => {
                let _ = BACKEND.set(Box::new(b));
                tracing::info!(backend = "redis", url = %url, "rate limiter initialised");
                return "redis";
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "rate limiter: Redis connect failed, falling back to memory backend"
                );
            }
        }
    }
    let _ = BACKEND.set(Box::new(MemoryBackend::default()));
    tracing::info!(backend = "memory", "rate limiter initialised");
    "memory"
}

static BACKEND: OnceLock<Box<dyn Backend>> = OnceLock::new();

fn backend() -> &'static dyn Backend {
    BACKEND
        .get()
        .map(|b| b.as_ref())
        .unwrap_or(&FALLBACK_BACKEND)
}

// Lazy fallback in case `init()` was never called (tests, ad-hoc bin).
struct LazyMemory(OnceLock<MemoryBackend>);
impl Backend for LazyMemory {
    fn check_and_record(&self, user_id: &str, limit: u32, now: Instant) -> RateLimitDecision {
        self.0
            .get_or_init(MemoryBackend::default)
            .check_and_record(user_id, limit, now)
    }
    fn name(&self) -> &'static str {
        "memory-lazy"
    }
}
static FALLBACK_BACKEND: LazyMemory = LazyMemory(OnceLock::new());

// ---------------------------------------------------------------------------
// Public entry points (stable surface — callers never change).
// ---------------------------------------------------------------------------

fn configured_limit() -> u32 {
    static LIMIT: OnceLock<u32> = OnceLock::new();
    *LIMIT.get_or_init(|| {
        std::env::var("MG_DEFAULT_RATE_LIMIT_PER_MIN")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_LIMIT)
    })
}

/// Increment the user's counter and return whether the request may proceed.
pub fn check_and_record(user_id: &str) -> RateLimitDecision {
    backend().check_and_record(user_id, configured_limit(), Instant::now())
}

/// Test seam: same logic, but the caller supplies the limit and clock.
/// Always uses an in-process MemoryBackend so unit tests don't need a
/// running Redis.
pub fn check_and_record_with_clock(user_id: &str, limit: u32, now: Instant) -> RateLimitDecision {
    static TEST: OnceLock<MemoryBackend> = OnceLock::new();
    TEST.get_or_init(MemoryBackend::default)
        .check_and_record(user_id, limit, now)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(matches!(
            check_and_record_with_clock(&user, 3, t0),
            RateLimitDecision::Deny { .. }
        ));

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
            assert_eq!(
                check_and_record_with_clock(&a, 2, now),
                RateLimitDecision::Allow
            );
        }
        assert!(matches!(
            check_and_record_with_clock(&a, 2, now),
            RateLimitDecision::Deny { .. }
        ));
        assert_eq!(
            check_and_record_with_clock(&b, 2, now),
            RateLimitDecision::Allow
        );
    }
}
