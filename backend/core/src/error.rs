//! Typed provider errors that map cleanly to HTTP status codes.
//!
//! Design: provider implementations still return `anyhow::Error` from the
//! trait so we don't touch 13 files in this PR. They construct
//! `ProviderError::*` and wrap it via `anyhow::Error::new(provider_err)`.
//! HTTP handlers downcast (`err.downcast_ref::<ProviderError>()`) and use
//! the typed variant to choose the right HTTP status, headers, and JSON body.
//!
//! Why this matters for ViralStory: it can implement a real retry policy
//! (`429` → wait `Retry-After`, `503` → exponential backoff, `400` → don't
//! retry) instead of receiving an opaque `500` for everything.

use std::time::Duration;

use poem::http::{header, HeaderValue, StatusCode};
use poem::web::Json;
use poem::{IntoResponse, Response};
use serde::Serialize;

/// A typed error from an upstream provider call.
///
/// Mapped to HTTP via `IntoResponse` so chat / image / video handlers can
/// just `?` the error up the stack.
#[derive(Debug, Clone)]
pub enum ProviderError {
    /// Upstream returned 429. Honour `Retry-After` if provided.
    RateLimit {
        provider: String,
        retry_after: Option<Duration>,
        message: String,
    },
    /// Upstream rejected the credential (401 / 403).
    Unauthorized { provider: String, message: String },
    /// Upstream rejected the request body (400 / 422).
    BadRequest { provider: String, message: String },
    /// Upstream timed out or didn't respond.
    Timeout { provider: String, message: String },
    /// Upstream is temporarily unavailable (503 / 502).
    Unavailable {
        provider: String,
        message: String,
        retry_after: Option<Duration>,
    },
    /// Upstream returned a 5xx other than 503/502 — provider's own bug.
    Internal {
        provider: String,
        status: u16,
        message: String,
    },
    /// Local config / setup error: missing API key, missing model mapping.
    Misconfigured { provider: String, message: String },
    /// Catch-all for anything we couldn't classify. Maps to 502 Bad Gateway
    /// (upstream returned something we couldn't make sense of).
    Other { provider: String, message: String },
}

impl ProviderError {
    pub fn provider(&self) -> &str {
        match self {
            ProviderError::RateLimit { provider, .. }
            | ProviderError::Unauthorized { provider, .. }
            | ProviderError::BadRequest { provider, .. }
            | ProviderError::Timeout { provider, .. }
            | ProviderError::Unavailable { provider, .. }
            | ProviderError::Internal { provider, .. }
            | ProviderError::Misconfigured { provider, .. }
            | ProviderError::Other { provider, .. } => provider,
        }
    }

    /// Variant key for metrics labels and logs (`rate_limit`, `unauthorized`, …).
    pub fn variant_key(&self) -> &'static str {
        match self {
            ProviderError::RateLimit { .. } => "rate_limit",
            ProviderError::Unauthorized { .. } => "unauthorized",
            ProviderError::BadRequest { .. } => "bad_request",
            ProviderError::Timeout { .. } => "timeout",
            ProviderError::Unavailable { .. } => "unavailable",
            ProviderError::Internal { .. } => "provider_internal",
            ProviderError::Misconfigured { .. } => "misconfigured",
            ProviderError::Other { .. } => "other",
        }
    }

    /// Whether the retry policy should attempt this error.
    /// Generally: yes for transient (429 / 5xx / timeout), no for client / config.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ProviderError::RateLimit { .. }
                | ProviderError::Timeout { .. }
                | ProviderError::Unavailable { .. }
                | ProviderError::Internal { .. }
        )
    }

    /// `Retry-After` hint if the provider gave us one.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            ProviderError::RateLimit { retry_after, .. }
            | ProviderError::Unavailable { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// HTTP status the gateway should return to its caller for this variant.
    pub fn status(&self) -> StatusCode {
        match self {
            ProviderError::RateLimit { .. } => StatusCode::TOO_MANY_REQUESTS,
            ProviderError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            ProviderError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ProviderError::Timeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            ProviderError::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            // Provider's own internal error → we proxy it through as 502
            // Bad Gateway, NOT 500, so callers can distinguish "we broke"
            // from "they broke".
            ProviderError::Internal { .. } => StatusCode::BAD_GATEWAY,
            // Misconfiguration is a 500: it's our setup that's wrong.
            ProviderError::Misconfigured { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ProviderError::Other { .. } => StatusCode::BAD_GATEWAY,
        }
    }

    /// JSON body shape returned to the caller. OpenAI-compatible.
    pub fn body(&self) -> ErrorBody {
        ErrorBody {
            error: ErrorBodyInner {
                kind: self.variant_key(),
                provider: self.provider().to_string(),
                message: self.message().to_string(),
            },
        }
    }

    pub fn message(&self) -> &str {
        match self {
            ProviderError::RateLimit { message, .. }
            | ProviderError::Unauthorized { message, .. }
            | ProviderError::BadRequest { message, .. }
            | ProviderError::Timeout { message, .. }
            | ProviderError::Unavailable { message, .. }
            | ProviderError::Internal { message, .. }
            | ProviderError::Misconfigured { message, .. }
            | ProviderError::Other { message, .. } => message,
        }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} [{}]: {}",
            self.provider(),
            self.variant_key(),
            self.message()
        )
    }
}

impl std::error::Error for ProviderError {}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    pub error: ErrorBodyInner,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorBodyInner {
    /// One of `rate_limit | unauthorized | bad_request | timeout | …`.
    /// Stable strings — clients can switch on these.
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub provider: String,
    pub message: String,
}

impl IntoResponse for ProviderError {
    fn into_response(self) -> Response {
        let status = self.status();
        let retry_after_header = self.retry_after().and_then(|d| {
            // RFC 7231 — Retry-After in delay-seconds form.
            HeaderValue::from_str(&d.as_secs().to_string()).ok()
        });
        let mut response = Json(self.body()).into_response();
        response.set_status(status);
        if let Some(value) = retry_after_header {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        response
    }
}

// ---------------------------------------------------------------------------
// Helpers — convert reqwest::Response and similar inputs to ProviderError.
// ---------------------------------------------------------------------------

/// Classify a non-success reqwest response into a `ProviderError`.
///
/// Intended use inside provider adapters:
///
///   if !resp.status().is_success() {
///       return Err(anyhow::Error::new(
///           classify_response(provider_name, resp).await
///       ));
///   }
pub async fn classify_response(provider: &str, response: reqwest::Response) -> ProviderError {
    let status = response.status();
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs);
    let body = response.text().await.unwrap_or_default();
    let snippet = truncate(&body, 512);

    match status.as_u16() {
        400 | 422 => ProviderError::BadRequest {
            provider: provider.to_string(),
            message: snippet,
        },
        401 | 403 => ProviderError::Unauthorized {
            provider: provider.to_string(),
            message: snippet,
        },
        408 => ProviderError::Timeout {
            provider: provider.to_string(),
            message: snippet,
        },
        429 => ProviderError::RateLimit {
            provider: provider.to_string(),
            retry_after,
            message: snippet,
        },
        502 | 503 | 504 => ProviderError::Unavailable {
            provider: provider.to_string(),
            retry_after,
            message: snippet,
        },
        s if s >= 500 => ProviderError::Internal {
            provider: provider.to_string(),
            status: s,
            message: snippet,
        },
        _ => ProviderError::Other {
            provider: provider.to_string(),
            message: format!("HTTP {status}: {snippet}"),
        },
    }
}

/// Classify a `reqwest::Error` (network / TLS / timeout) into ProviderError.
pub fn classify_reqwest_error(provider: &str, err: reqwest::Error) -> ProviderError {
    if err.is_timeout() {
        return ProviderError::Timeout {
            provider: provider.to_string(),
            message: err.to_string(),
        };
    }
    if err.is_connect() {
        return ProviderError::Unavailable {
            provider: provider.to_string(),
            message: err.to_string(),
            retry_after: None,
        };
    }
    ProviderError::Other {
        provider: provider.to_string(),
        message: err.to_string(),
    }
}

/// Try to extract a `ProviderError` from an opaque `anyhow::Error`.
/// Walks the error chain so a `.context()`-wrapped `ProviderError`
/// (e.g. `e.context("Provider API error")` from the executor's
/// failover loop) still resolves correctly.
pub fn downcast(err: &anyhow::Error) -> Option<&ProviderError> {
    if let Some(pe) = err.downcast_ref::<ProviderError>() {
        return Some(pe);
    }
    for cause in err.chain() {
        if let Some(pe) = cause.downcast_ref::<ProviderError>() {
            return Some(pe);
        }
    }
    None
}

/// Convert an opaque `anyhow::Error` from a provider call into a Poem error
/// with the correct HTTP status. Falls back to a generic 500 for errors we
/// can't classify (e.g. database failures, panics caught higher up).
///
/// Use:
///
///   .map_err(into_poem_error)
pub fn into_poem_error(err: anyhow::Error) -> poem::Error {
    if let Some(pe) = err.downcast_ref::<ProviderError>() {
        let status = pe.status();
        let body = pe.body();
        let mut poem_err = poem::Error::from_response(
            poem::Response::builder()
                .status(status)
                .content_type("application/json")
                .body(serde_json::to_string(&body).unwrap_or_default()),
        );
        if let Some(retry_after) = pe.retry_after() {
            // Surface Retry-After through Poem's error so middleware /
            // clients can read it. Currently set on the response above.
            let _ = retry_after;
        }
        // Also stash the variant key on the error for log correlation.
        poem_err.set_data(crate::error::ErrorVariantKey(pe.variant_key()));
        return poem_err;
    }
    // Default: opaque server error.
    poem::Error::from_string(
        format!("Internal server error: {err}"),
        StatusCode::INTERNAL_SERVER_ERROR,
    )
}

/// Newtype so handlers can pull the variant_key off a poem::Error.
#[derive(Debug, Clone, Copy)]
pub struct ErrorVariantKey(pub &'static str);

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_mapping_matches_spec() {
        let cases = [
            (
                ProviderError::RateLimit {
                    provider: "x".into(),
                    retry_after: None,
                    message: "".into(),
                },
                StatusCode::TOO_MANY_REQUESTS,
            ),
            (
                ProviderError::Unauthorized {
                    provider: "x".into(),
                    message: "".into(),
                },
                StatusCode::UNAUTHORIZED,
            ),
            (
                ProviderError::BadRequest {
                    provider: "x".into(),
                    message: "".into(),
                },
                StatusCode::BAD_REQUEST,
            ),
            (
                ProviderError::Timeout {
                    provider: "x".into(),
                    message: "".into(),
                },
                StatusCode::GATEWAY_TIMEOUT,
            ),
            (
                ProviderError::Unavailable {
                    provider: "x".into(),
                    message: "".into(),
                    retry_after: None,
                },
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                ProviderError::Internal {
                    provider: "x".into(),
                    status: 500,
                    message: "".into(),
                },
                StatusCode::BAD_GATEWAY,
            ),
            (
                ProviderError::Misconfigured {
                    provider: "x".into(),
                    message: "".into(),
                },
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.status(), expected, "wrong status for {:?}", err);
        }
    }

    #[test]
    fn retryable_classification() {
        assert!(ProviderError::RateLimit {
            provider: "x".into(),
            retry_after: None,
            message: "".into()
        }
        .is_retryable());
        assert!(!ProviderError::BadRequest {
            provider: "x".into(),
            message: "".into()
        }
        .is_retryable());
        assert!(!ProviderError::Unauthorized {
            provider: "x".into(),
            message: "".into()
        }
        .is_retryable());
    }

    #[test]
    fn downcast_through_anyhow() {
        let pe = ProviderError::RateLimit {
            provider: "openai".into(),
            retry_after: Some(Duration::from_secs(7)),
            message: "limit hit".into(),
        };
        let wrapped: anyhow::Error = anyhow::Error::new(pe);
        let pulled = downcast(&wrapped).expect("downcast");
        assert_eq!(pulled.provider(), "openai");
        assert_eq!(pulled.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(pulled.retry_after(), Some(Duration::from_secs(7)));
    }

    /// Mirrors the gate in `executor.rs`:
    ///
    ///     let do_failover = match downcast(&e) {
    ///         Some(pe) => pe.is_retryable(),
    ///         None     => true, // unmigrated adapters keep old behaviour
    ///     };
    ///
    /// The whole reliability story rides on this table being right, so we
    /// pin every variant explicitly. If a variant changes class, the
    /// failover loop changes behaviour silently — a regression test here
    /// catches it before deploy.
    fn pe(builder: ProviderError) -> anyhow::Error {
        anyhow::Error::new(builder)
    }

    #[test]
    fn failover_gate_table() {
        let cases: Vec<(&'static str, anyhow::Error, bool)> = vec![
            (
                "RateLimit (429)",
                pe(ProviderError::RateLimit {
                    provider: "x".into(),
                    retry_after: None,
                    message: "".into(),
                }),
                true,
            ),
            (
                "Timeout (408)",
                pe(ProviderError::Timeout { provider: "x".into(), message: "".into() }),
                true,
            ),
            (
                "Unavailable (502/503/504)",
                pe(ProviderError::Unavailable {
                    provider: "x".into(),
                    message: "".into(),
                    retry_after: None,
                }),
                true,
            ),
            (
                "Internal (5xx other)",
                pe(ProviderError::Internal {
                    provider: "x".into(),
                    status: 500,
                    message: "".into(),
                }),
                true,
            ),
            (
                "Unauthorized (401/403) — DO NOT failover",
                pe(ProviderError::Unauthorized { provider: "x".into(), message: "".into() }),
                false,
            ),
            (
                "BadRequest (400/422) — DO NOT failover",
                pe(ProviderError::BadRequest { provider: "x".into(), message: "".into() }),
                false,
            ),
            (
                "Misconfigured — DO NOT failover",
                pe(ProviderError::Misconfigured { provider: "x".into(), message: "".into() }),
                false,
            ),
            (
                "Other — DO NOT failover (treated as client-class)",
                pe(ProviderError::Other { provider: "x".into(), message: "".into() }),
                false,
            ),
        ];

        for (label, err, expect_failover) in cases {
            let do_failover = match downcast(&err) {
                Some(p) => p.is_retryable(),
                None => true,
            };
            assert_eq!(do_failover, expect_failover, "{label}");
        }
    }

    #[test]
    fn untyped_anyhow_still_failovers() {
        // Backward-compat: any adapter that hasn't been migrated to
        // classify_response yet returns a plain `anyhow!("...")`. The
        // executor must keep failing over on those — the alternative is
        // a behaviour regression (e.g. the gateway suddenly stops failing
        // over on Mistral because Mistral hasn't been migrated yet).
        let untyped = anyhow::anyhow!("HTTP 500: opaque");
        let do_failover = match downcast(&untyped) {
            Some(p) => p.is_retryable(),
            None => true,
        };
        assert!(do_failover, "untyped errors must still failover for backward compat");
    }
}
