//! Observability — request_id middleware, structured logging, per-route metrics.
//!
//! Wiring:
//!   - `init_tracing()` in `main()` configures the global tracing subscriber.
//!     Default output: human-readable to stderr. Production override: set
//!     `LOG_FORMAT=json` for one-line-per-event JSON suitable for Loki / Datadog.
//!   - `RequestContext` middleware wraps every HTTP request:
//!       1. accepts incoming `x-request-id` header or generates a UUIDv4
//!       2. opens a tracing span carrying request_id, method, path
//!       3. records the request in metrics (in-flight gauge,
//!          per-(route, method, status) counter and latency histogram)
//!       4. echoes `x-request-id` back so callers can correlate
//!
//! All call-site logging that runs inside an HTTP request inherits the
//! request_id span field automatically.

use std::time::Instant;

use poem::http::HeaderValue;
use poem::{Endpoint, IntoResponse, Middleware, Request, Response, Result};
use tracing::Instrument;
use uuid::Uuid;

use crate::metrics;

const REQUEST_ID_HEADER: &str = "x-request-id";

// ---------------------------------------------------------------------------
// init_tracing — call once from main()
// ---------------------------------------------------------------------------

/// Initialise the global tracing subscriber.
///
/// Reads from env:
///   - `MG_RUST_LOG` (default `info`) — `tracing-subscriber` env-filter syntax
///   - `MG_LOG_FORMAT` — `json` for structured JSON, anything else (default)
///     for the human-readable formatter
pub fn init_tracing() {
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::EnvFilter;

    let env_filter = EnvFilter::try_from_env("MG_RUST_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn,hyper=warn"));

    let format = std::env::var("MG_LOG_FORMAT").unwrap_or_default();

    let registry = tracing_subscriber::registry().with(env_filter);

    if format.eq_ignore_ascii_case("json") {
        registry
            .with(
                fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(false)
                    .with_target(true)
                    .flatten_event(true),
            )
            .init();
    } else {
        registry
            .with(fmt::layer().with_target(false).compact())
            .init();
    }
}

// ---------------------------------------------------------------------------
// RequestContext middleware
// ---------------------------------------------------------------------------

/// Inject request_id, open a tracing span, record per-route metrics.
pub struct RequestContext;

impl<E: Endpoint> Middleware<E> for RequestContext {
    type Output = RequestContextEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        RequestContextEndpoint { inner: ep }
    }
}

pub struct RequestContextEndpoint<E> {
    inner: E,
}

impl<E: Endpoint> Endpoint for RequestContextEndpoint<E> {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        // 1. Resolve / mint request_id and stash it in extensions for handlers.
        let request_id = req
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        req.extensions_mut().insert(RequestId(request_id.clone()));

        let method = req.method().to_string();
        let route = matched_route(&req);
        let started = Instant::now();

        // 2. In-flight gauge increments for the duration of the request.
        metrics::REQUESTS_IN_FLIGHT.inc();
        let _guard = scopeguard::guard((), |_| {
            metrics::REQUESTS_IN_FLIGHT.dec();
        });

        // 3. Open a span carrying the request fields. All log events emitted
        //    by handlers (and the libraries they call) inherit these fields.
        let span = tracing::info_span!(
            "http.request",
            request_id = %request_id,
            method = %method,
            route = %route,
        );

        let outcome = self.inner.call(req).instrument(span.clone()).await;
        let elapsed = started.elapsed();

        // 4. Convert handler result to Response, attach request_id header,
        //    record per-route metrics, log a one-line summary.
        let mut response: Response = match outcome {
            Ok(out) => out.into_response(),
            Err(err) => {
                // Surface the error class to logs/metrics, then convert.
                let resp = err.into_response();
                metrics::HTTP_REQUESTS_ERRORS.inc();
                resp
            }
        };

        let status = response.status().as_u16();
        let status_label = format!("{status}");

        if let Ok(value) = HeaderValue::from_str(&request_id) {
            response.headers_mut().insert(REQUEST_ID_HEADER, value);
        }

        // Per-route metrics — the labelled vectors live in metrics.rs
        metrics::HTTP_REQUESTS_BY_ROUTE
            .with_label_values(&[&route, &method, &status_label])
            .inc();
        metrics::HTTP_REQUEST_DURATION_BY_ROUTE
            .with_label_values(&[&route, &method, &status_label])
            .observe(elapsed.as_secs_f64());
        metrics::HTTP_REQUESTS_TOTAL.inc();
        metrics::REQUEST_DURATION.observe(elapsed.as_secs_f64());
        if status >= 500 {
            metrics::HTTP_REQUESTS_ERRORS.inc();
        }

        // Single structured access-log line per request. The level is INFO for
        // 2xx/3xx, WARN for 4xx, ERROR for 5xx — easy to alert on.
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        if status >= 500 {
            tracing::error!(
                parent: &span,
                status,
                elapsed_ms,
                "request finished"
            );
        } else if status >= 400 {
            tracing::warn!(
                parent: &span,
                status,
                elapsed_ms,
                "request finished"
            );
        } else {
            tracing::info!(
                parent: &span,
                status,
                elapsed_ms,
                "request finished"
            );
        }

        Ok(response)
    }
}

/// Wrapper for the request_id stashed in request extensions so handlers can
/// read it without colliding with raw `String`s in extensions.
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Best-effort route identifier for metrics labels. Poem doesn't surface the
/// matched template directly here, so we collapse to method-prefixed path.
/// Bounded cardinality is critical for Prometheus — we strip query strings
/// and replace UUIDs / numeric IDs with `:id` to avoid label explosion.
fn matched_route(req: &Request) -> String {
    let path = req.uri().path();
    collapse_route(path)
}

/// Collapse high-cardinality path segments (UUIDs, numeric IDs) to `:id`.
fn collapse_route(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for segment in path.split('/') {
        if segment.is_empty() {
            continue;
        }
        out.push('/');
        if looks_like_id(segment) {
            out.push_str(":id");
        } else {
            out.push_str(segment);
        }
    }
    if out.is_empty() {
        out.push('/');
    }
    out
}

fn looks_like_id(s: &str) -> bool {
    // UUID
    if s.len() == 36 && s.matches('-').count() == 4 {
        return true;
    }
    // All-numeric (DB ids)
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_uuid() {
        assert_eq!(
            collapse_route("/v1/videos/jobs/550e8400-e29b-41d4-a716-446655440000/sora-1"),
            "/v1/videos/jobs/:id/sora-1"
        );
    }

    #[test]
    fn collapses_numeric_id() {
        assert_eq!(collapse_route("/v1/services/42"), "/v1/services/:id");
    }

    #[test]
    fn leaves_named_segments_alone() {
        assert_eq!(
            collapse_route("/v1/chat/completions"),
            "/v1/chat/completions"
        );
    }

    #[test]
    fn handles_root() {
        assert_eq!(collapse_route("/"), "/");
    }
}
