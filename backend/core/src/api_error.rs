//! OpenAI-shape error responses for the public API.
//!
//! When MawiGateway returns a non-2xx, the body matches OpenAI's
//! envelope so existing OpenAI / Anthropic SDKs unmarshal it
//! correctly:
//!
//! ```json
//! {
//!   "error": {
//!     "message": "Service 'gpt-4o' not found",
//!     "type": "not_found_error",
//!     "code": "service_not_found",
//!     "param": "service"
//!   }
//! }
//! ```
//!
//! Without this envelope, Python's `openai.NotFoundError` raises with an
//! empty `.message` and a confusing `body` payload — so drop-in compat
//! is incomplete. This module supplies the types + helpers for handlers
//! to build the right shape with one line.
//!
//! ## When to use
//!
//! Every public endpoint that already returns `Json<String>` for errors
//! should switch to `Json<OpenAiErrorResponse>`. The chat endpoint is
//! the highest leverage (SDK users hit that one); CRUD endpoints
//! (services, providers, models, api-keys) come next.
//!
//! ## When NOT to use
//!
//! Internal-only endpoints (`/health`, `/metrics`, `/spec`,
//! `/swagger-ui`) are for load balancers and humans, not API callers.
//! Leave them alone.

use serde::{Deserialize, Serialize};

/// Top-level envelope. The body of every non-2xx response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct OpenAiErrorResponse {
    pub error: OpenAiError,
}

/// The error object inside the envelope. Field names match OpenAI's
/// schema exactly so `client.error.message` etc. work in every SDK
/// without per-language adapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct OpenAiError {
    /// Human-readable description. SDKs surface this as `.message`.
    pub message: String,

    /// Coarse error class. SDKs use this to pick the exception type
    /// (`openai.BadRequestError` vs `openai.AuthenticationError`).
    /// One of the constants in [`error_type`].
    #[serde(rename = "type")]
    pub kind: String,

    /// Optional fine-grained code (`service_not_found`,
    /// `invalid_api_key`, …). Useful for programmatic branching but not
    /// required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// Which request field the error refers to, if any. Mirrors OpenAI:
    /// they include `param: "model"` when the issue is the model field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
}

/// Stable string constants for the `type` field. Match OpenAI's set so
/// SDKs map them to their typed exception classes correctly.
pub mod error_type {
    /// 4xx — request shape, missing fields, bad values.
    pub const INVALID_REQUEST: &str = "invalid_request_error";
    /// 401 — missing or rejected credentials.
    pub const AUTHENTICATION: &str = "authentication_error";
    /// 403 — authenticated but forbidden from this resource.
    pub const PERMISSION: &str = "permission_error";
    /// 404 — resource not found.
    pub const NOT_FOUND: &str = "not_found_error";
    /// 409 — uniqueness / state conflict.
    pub const CONFLICT: &str = "invalid_request_error"; // OpenAI lumps 409 here
    /// 429 — over the rate limit or quota.
    pub const RATE_LIMIT: &str = "rate_limit_error";
    /// 5xx — server-side error.
    pub const API: &str = "api_error";
    /// 503 — temporarily unavailable.
    pub const SERVICE_UNAVAILABLE: &str = "service_unavailable_error";
}

impl OpenAiError {
    /// Build an error envelope. Use the constants in [`error_type`] for
    /// the `kind` parameter so SDKs map to the right exception class.
    pub fn new(message: impl Into<String>, kind: &str) -> OpenAiErrorResponse {
        OpenAiErrorResponse {
            error: OpenAiError {
                message: message.into(),
                kind: kind.to_string(),
                code: None,
                param: None,
            },
        }
    }

    /// Like [`new`] but with a fine-grained `code`.
    pub fn with_code(
        message: impl Into<String>,
        kind: &str,
        code: impl Into<String>,
    ) -> OpenAiErrorResponse {
        OpenAiErrorResponse {
            error: OpenAiError {
                message: message.into(),
                kind: kind.to_string(),
                code: Some(code.into()),
                param: None,
            },
        }
    }

    /// Builder finisher — attach a `param` when the error refers to a
    /// specific request field.
    pub fn for_param(
        message: impl Into<String>,
        kind: &str,
        param: impl Into<String>,
    ) -> OpenAiErrorResponse {
        OpenAiErrorResponse {
            error: OpenAiError {
                message: message.into(),
                kind: kind.to_string(),
                code: None,
                param: Some(param.into()),
            },
        }
    }
}

/// Convenience: pick the right `kind` from a status code. Use when you
/// have an HTTP status from a downstream call and want to surface it
/// to the client with a sensible class.
pub fn kind_for_status(status: u16) -> &'static str {
    match status {
        400 | 422 => error_type::INVALID_REQUEST,
        401 => error_type::AUTHENTICATION,
        403 => error_type::PERMISSION,
        404 => error_type::NOT_FOUND,
        409 => error_type::CONFLICT,
        429 => error_type::RATE_LIMIT,
        503 => error_type::SERVICE_UNAVAILABLE,
        500..=599 => error_type::API,
        _ => error_type::INVALID_REQUEST,
    }
}

// ---------------------------------------------------------------------------
// Poem integration. mawi-core already depends on poem (auth middleware,
// utils), so we can ship the `poem::Error` constructors here so any crate
// in the workspace can return OpenAI-shape errors without rebuilding the
// envelope construction. See gateway/src/openai_err.rs for the equivalent
// gateway-local helpers — those just delegate to the constants below.
// ---------------------------------------------------------------------------

/// Wrap an [`OpenAiErrorResponse`] in a `poem::Error` with the given
/// status. Body is a JSON document; Content-Type is set to
/// `application/json` so SDKs unmarshal it correctly.
pub fn poem_error_from_envelope(
    status: poem::http::StatusCode,
    env: OpenAiErrorResponse,
) -> poem::Error {
    use poem::http::header;
    use poem::Response;
    let body = serde_json::to_vec(&env).unwrap_or_else(|_| {
        br#"{"error":{"message":"internal serialization failure","type":"api_error"}}"#.to_vec()
    });
    let resp = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body);
    poem::Error::from_response(resp)
}

/// Authentication-error helper for `mawi-core` callers that don't want
/// to build the envelope themselves. Equivalent to
/// `gateway::openai_err::unauthorized` but available in this crate so
/// `auth/utils.rs` and similar mawi-core sites can return the right
/// shape directly.
pub fn poem_unauthorized(message: impl Into<String>) -> poem::Error {
    poem_error_from_envelope(
        poem::http::StatusCode::UNAUTHORIZED,
        OpenAiError::new(message, error_type::AUTHENTICATION),
    )
}

/// Internal-server-error helper.
pub fn poem_internal(message: impl Into<String>) -> poem::Error {
    poem_error_from_envelope(
        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
        OpenAiError::new(message, error_type::API),
    )
}
