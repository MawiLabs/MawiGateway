//! Helper for returning OpenAI-shape error responses from `poem` handlers
//! that use `poem::Result<T>` (not `ApiResponse` enums like `ChatResponse`).
//!
//! Most CRUD endpoints in `api.rs` were written before [`mawi_core::api_error`]
//! existed and use `poem::error::Error::from_string(...)`. That produces a
//! `text/plain` body with no structure — fine for humans, useless for the
//! OpenAI / Anthropic SDKs that try to parse `{ error: { message, type } }`.
//!
//! This module gives those handlers a one-liner upgrade:
//!
//! ```ignore
//! use crate::openai_err;
//! return Err(openai_err::not_found("Service 'x' not found", Some("service")));
//! ```
//!
//! The result is the right HTTP status + a JSON body the SDKs unmarshal
//! into their typed exception classes.

use mawi_core::api_error::{error_type, OpenAiError, OpenAiErrorResponse};
use poem::http::{header, StatusCode};
use poem::Response;

/// Wrap an [`OpenAiErrorResponse`] in a `poem::Error` with the given status.
fn from_envelope(status: StatusCode, env: OpenAiErrorResponse) -> poem::Error {
    // Serialize the envelope. If JSON encoding ever fails (it shouldn't),
    // fall back to a tiny static body so the request still gets a
    // well-shaped response rather than a 500 from the error pipeline.
    let body = serde_json::to_vec(&env).unwrap_or_else(|_| {
        br#"{"error":{"message":"internal serialization failure","type":"api_error"}}"#
            .to_vec()
    });
    // poem::Response::builder().status(...) sets the status, .body() consumes
    // the builder and returns a Response — no separate status_mut() needed.
    let resp = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body);
    poem::Error::from_response(resp)
}

// ---------------------------------------------------------------------------
// Convenience constructors. One per common HTTP/SDK error class. Use these
// instead of building envelopes by hand to keep the type-string usage
// consistent across the codebase.
// ---------------------------------------------------------------------------

pub fn bad_request(message: impl Into<String>, param: Option<&str>) -> poem::Error {
    let env = match param {
        Some(p) => OpenAiError::for_param(message, error_type::INVALID_REQUEST, p),
        None => OpenAiError::new(message, error_type::INVALID_REQUEST),
    };
    from_envelope(StatusCode::BAD_REQUEST, env)
}

pub fn unauthorized(message: impl Into<String>) -> poem::Error {
    from_envelope(
        StatusCode::UNAUTHORIZED,
        OpenAiError::new(message, error_type::AUTHENTICATION),
    )
}

pub fn forbidden(message: impl Into<String>) -> poem::Error {
    from_envelope(
        StatusCode::FORBIDDEN,
        OpenAiError::new(message, error_type::PERMISSION),
    )
}

pub fn not_found(message: impl Into<String>, param: Option<&str>) -> poem::Error {
    let env = match param {
        Some(p) => OpenAiError::for_param(message, error_type::NOT_FOUND, p),
        None => OpenAiError::new(message, error_type::NOT_FOUND),
    };
    from_envelope(StatusCode::NOT_FOUND, env)
}

pub fn conflict(message: impl Into<String>, code: Option<&str>) -> poem::Error {
    let env = match code {
        Some(c) => OpenAiError::with_code(message, error_type::CONFLICT, c),
        None => OpenAiError::new(message, error_type::CONFLICT),
    };
    from_envelope(StatusCode::CONFLICT, env)
}

pub fn rate_limited(message: impl Into<String>) -> poem::Error {
    from_envelope(
        StatusCode::TOO_MANY_REQUESTS,
        OpenAiError::new(message, error_type::RATE_LIMIT),
    )
}

pub fn internal(message: impl Into<String>) -> poem::Error {
    from_envelope(
        StatusCode::INTERNAL_SERVER_ERROR,
        OpenAiError::new(message, error_type::API),
    )
}
