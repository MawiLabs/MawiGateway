//! Idempotency-key cache for retry-safe inference POSTs (#41).
//!
//! Implements the Stripe pattern: the client picks an `Idempotency-Key`
//! and includes it on a POST. The first request executes normally and
//! the gateway records `(status, body)`. A retry with the same key
//! returns the recorded response without re-executing — so a network
//! blip can't double-bill expensive provider calls.
//!
//! Scope of this module: storage and decision logic. Per-endpoint
//! integration (header parsing, response replay) is wired into each
//! POST handler that opts in. Currently `/v1/chat/completions`
//! non-streaming; image / audio / video follow in a separate PR.
//!
//! Validity rules:
//! - Key length 8..=256, ASCII alphanumeric + `_-.` only. Anything
//!   outside is rejected at parse time so the DB is never asked to
//!   store garbage.
//! - Same `(user_id, key)` with a different request body → 409
//!   "idempotency-key reuse with different request" rather than
//!   silently returning the old response.
//! - TTL is `IDEMPOTENCY_TTL_SECS` (default 86_400 — 24h, matches Stripe).
//! - Streaming endpoints are NOT covered: their response is incremental
//!   and can't be cached usefully.

use anyhow::{anyhow, Result};
use poem::http::StatusCode;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::time::{Duration, SystemTime};

const MIN_KEY_LEN: usize = 8;
const MAX_KEY_LEN: usize = 256;
const DEFAULT_TTL_SECS: i64 = 86_400;

/// Result of an idempotency check.
#[derive(Debug)]
pub enum IdempotencyDecision {
    /// No `Idempotency-Key` header — caller must execute normally
    /// and skip recording.
    NoKey,

    /// Fresh key for this user. Caller should execute, then call
    /// [`record`] with the response.
    Fresh,

    /// Replay of a prior request with the same body. Return the
    /// stored response unchanged.
    Cached {
        status_code: i32,
        response_body: Vec<u8>,
        content_type: Option<String>,
    },

    /// Same key reused with a different request body. Caller must
    /// reject with HTTP 409 — see [`IdempotencyError::Mismatch`].
    Mismatch,
}

/// Errors callers may need to surface as HTTP status codes.
#[derive(Debug)]
pub enum IdempotencyError {
    /// Header value present but malformed (length / chars). Caller
    /// should return HTTP 400 with the inner message.
    InvalidHeader(String),
    /// `(user_id, key)` exists but request body differs. Caller
    /// should return HTTP 409 with a clear message.
    Mismatch,
}

impl std::fmt::Display for IdempotencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHeader(m) => write!(f, "invalid Idempotency-Key header: {}", m),
            Self::Mismatch => write!(f, "idempotency-key reuse with a different request body"),
        }
    }
}

impl std::error::Error for IdempotencyError {}

/// Validate an `Idempotency-Key` header value. Returns the trimmed
/// key on success.
pub fn parse_key(raw: &str) -> std::result::Result<String, IdempotencyError> {
    let trimmed = raw.trim();
    if trimmed.len() < MIN_KEY_LEN || trimmed.len() > MAX_KEY_LEN {
        return Err(IdempotencyError::InvalidHeader(format!(
            "length must be between {} and {} chars, got {}",
            MIN_KEY_LEN,
            MAX_KEY_LEN,
            trimmed.len()
        )));
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Err(IdempotencyError::InvalidHeader(
            "only ASCII alphanumeric + `_-.` allowed".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Read + validate the optional `Idempotency-Key` header from a Poem
/// request. Returns:
/// - `Ok(None)` if the header is absent (idempotency is opt-in).
/// - `Ok(Some(key))` if the header is well-formed.
/// - `Err(poem::Error)` with HTTP 400 if present but malformed —
///   suitable for a handler to return directly via `?`.
pub fn header_from_request(req: &poem::Request) -> poem::Result<Option<String>> {
    let raw = match req.headers().get("Idempotency-Key") {
        Some(v) => v,
        None => return Ok(None),
    };
    let s = raw.to_str().map_err(|e| {
        poem::Error::from_string(
            format!("Idempotency-Key not valid UTF-8: {}", e),
            StatusCode::BAD_REQUEST,
        )
    })?;
    parse_key(s)
        .map(Some)
        .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::BAD_REQUEST))
}

/// SHA-256 of `method || \n || path || \n || body`. Caller passes a
/// canonical representation; the gateway is the only consumer of the
/// hash so format stability is up to us.
pub fn hash_request(method: &str, path: &str, body: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(method.as_bytes());
    h.update(b"\n");
    h.update(path.as_bytes());
    h.update(b"\n");
    h.update(body);
    hex::encode(h.finalize())
}

fn ttl_secs() -> i64 {
    std::env::var("MG_IDEMPOTENCY_TTL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|n: &i64| *n > 0)
        .unwrap_or(DEFAULT_TTL_SECS)
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Check whether `(user_id, key)` already has a recorded response.
///
/// - `NoKey` if `key` is None (caller didn't supply the header).
/// - `Cached` if the same body was already recorded.
/// - `Mismatch` if the key exists but the body hash differs.
/// - `Fresh` if no row exists yet — caller proceeds and calls
///   [`record`] after execution.
///
/// Expired rows (`expires_at < now`) are treated as if absent. They
/// stay in the table until the cleanup task removes them; the check
/// query already filters them out.
pub async fn check(
    pool: &PgPool,
    user_id: &str,
    key: Option<&str>,
    request_hash: &str,
) -> Result<IdempotencyDecision> {
    let Some(key) = key else {
        return Ok(IdempotencyDecision::NoKey);
    };

    let now = now_secs();
    let row: Option<(String, i32, Vec<u8>, Option<String>)> = sqlx::query_as(
        "SELECT request_hash, status_code, response_body, content_type \
         FROM idempotency_keys \
         WHERE user_id = $1 AND key = $2 AND expires_at > $3",
    )
    .bind(user_id)
    .bind(key)
    .bind(now)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow!("idempotency check query failed: {}", e))?;

    let Some((stored_hash, status_code, response_body, content_type)) = row else {
        return Ok(IdempotencyDecision::Fresh);
    };

    if stored_hash != request_hash {
        return Ok(IdempotencyDecision::Mismatch);
    }

    Ok(IdempotencyDecision::Cached {
        status_code,
        response_body,
        content_type,
    })
}

/// Persist a freshly-computed response under `(user_id, key)`.
///
/// `INSERT ... ON CONFLICT DO NOTHING` so two concurrent requests that
/// raced past `check` (both seeing `Fresh`) end up with one persisted
/// row instead of an error. The loser's compute is wasted but the
/// stored response is consistent — same outcome a Stripe-style gateway
/// gives.
pub async fn record(
    pool: &PgPool,
    user_id: &str,
    key: &str,
    request_hash: &str,
    status_code: i32,
    response_body: &[u8],
    content_type: Option<&str>,
) -> Result<()> {
    let now = now_secs();
    let expires_at = now.saturating_add(ttl_secs());

    sqlx::query(
        "INSERT INTO idempotency_keys \
            (user_id, key, request_hash, status_code, response_body, content_type, created_at, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         ON CONFLICT (user_id, key) DO NOTHING",
    )
    .bind(user_id)
    .bind(key)
    .bind(request_hash)
    .bind(status_code)
    .bind(response_body)
    .bind(content_type)
    .bind(now)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|e| anyhow!("idempotency record query failed: {}", e))?;

    Ok(())
}

/// Background task that periodically deletes expired idempotency rows.
///
/// Without this, the `idempotency_keys` table grows by every POST that
/// supplies a header — at, say, 100 keyed requests per minute over 24h
/// of TTL, the table accumulates ~144k rows steady-state, then ~144k
/// of dead-row bloat per cleanup interval until vacuum reclaims them.
///
/// The default interval is 1 hour (`MG_IDEMPOTENCY_CLEANUP_INTERVAL_SECS`,
/// override for shorter retention or low-throughput deployments). The
/// task uses `MissedTickBehavior::Delay` so a slow cleanup query
/// doesn't queue up overlapping sweepers.
pub fn start_cleanup_task(pool: PgPool) {
    let interval_secs = std::env::var("MG_IDEMPOTENCY_CLEANUP_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|n: &u64| *n > 0)
        .unwrap_or(3600);

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // First tick fires immediately — skip it so a gateway that
        // restarts often doesn't hammer the DELETE on every boot.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let now = now_secs();
            match sqlx::query("DELETE FROM idempotency_keys WHERE expires_at < $1")
                .bind(now)
                .execute(&pool)
                .await
            {
                Ok(r) if r.rows_affected() > 0 => {
                    tracing::info!(
                        deleted = r.rows_affected(),
                        "idempotency cleanup: removed expired rows"
                    );
                }
                Ok(_) => {
                    tracing::debug!("idempotency cleanup: no expired rows");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "idempotency cleanup query failed");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_key_accepts_valid() {
        assert_eq!(parse_key("abcd-1234").unwrap(), "abcd-1234");
        assert_eq!(parse_key("  test_key.v1  ").unwrap(), "test_key.v1");
        // Right at the boundary.
        let exactly_8 = "12345678";
        assert_eq!(parse_key(exactly_8).unwrap(), exactly_8);
    }

    #[test]
    fn parse_key_rejects_too_short() {
        let err = parse_key("short").unwrap_err();
        assert!(matches!(err, IdempotencyError::InvalidHeader(_)));
    }

    #[test]
    fn parse_key_rejects_too_long() {
        let huge = "a".repeat(MAX_KEY_LEN + 1);
        let err = parse_key(&huge).unwrap_err();
        assert!(matches!(err, IdempotencyError::InvalidHeader(_)));
    }

    #[test]
    fn parse_key_rejects_invalid_chars() {
        let err = parse_key("with spaces").unwrap_err();
        assert!(matches!(err, IdempotencyError::InvalidHeader(_)));
        let err = parse_key("emoji-🚀-key").unwrap_err();
        assert!(matches!(err, IdempotencyError::InvalidHeader(_)));
        let err = parse_key("forward/slash-bad").unwrap_err();
        assert!(matches!(err, IdempotencyError::InvalidHeader(_)));
    }

    #[test]
    fn hash_request_stable_for_same_input() {
        let a = hash_request("POST", "/v1/chat/completions", br#"{"x":1}"#);
        let b = hash_request("POST", "/v1/chat/completions", br#"{"x":1}"#);
        assert_eq!(a, b);
    }

    #[test]
    fn hash_request_changes_with_body() {
        let a = hash_request("POST", "/v1/chat/completions", br#"{"x":1}"#);
        let b = hash_request("POST", "/v1/chat/completions", br#"{"x":2}"#);
        assert_ne!(a, b);
    }

    #[test]
    fn hash_request_changes_with_path() {
        let a = hash_request("POST", "/v1/chat/completions", b"x");
        let b = hash_request("POST", "/v1/images/generations", b"x");
        assert_ne!(a, b);
    }

    #[test]
    fn ttl_default_is_24h() {
        std::env::remove_var("MG_IDEMPOTENCY_TTL_SECS");
        assert_eq!(ttl_secs(), 86_400);
    }
}
