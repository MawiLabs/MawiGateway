use crate::executor::Executor;
use crate::idempotency::{self, IdempotencyDecision};
use mawi_core::types::{ImageGenerationRequest, ImageGenerationResponse};
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Json},
};
use sqlx::PgPool;
use std::sync::Arc;

#[handler]
pub async fn image_generations(
    req: &poem::Request,
    Data(executor): Data<&Arc<Executor>>,
    Data(pool): Data<&PgPool>,
    Json(request): Json<ImageGenerationRequest>,
) -> poem::Result<Json<ImageGenerationResponse>> {
    // Extract user_id from session (injected by AuthMiddleware)
    let user = req
        .extensions()
        .get::<mawi_core::auth::User>()
        .ok_or_else(|| {
            poem::Error::from_string("Authentication required", StatusCode::UNAUTHORIZED)
        })?;
    let user_id = user.id.clone();

    // Idempotency-key handling — same pattern as `/v1/chat/completions`
    // (#41). Image generation is among the most expensive provider
    // calls per request, so retry-safety here is high-value.
    let idem_key = parse_idempotency_header(req)?;
    let body_bytes = serde_json::to_vec(&request).map_err(|e| {
        poem::Error::from_string(
            format!("could not re-serialise request for idempotency hash: {}", e),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    let request_hash = idempotency::hash_request("POST", "/v1/images/generations", &body_bytes);

    if let Some(ref key) = idem_key {
        match idempotency::check(pool, &user_id, Some(key), &request_hash).await {
            Ok(IdempotencyDecision::Cached { response_body, .. }) => {
                if let Ok(resp) = serde_json::from_slice::<ImageGenerationResponse>(&response_body)
                {
                    return Ok(Json(resp));
                }
                tracing::warn!(
                    user_id = %user_id,
                    "idempotency cache hit but stored body did not deserialise; re-executing"
                );
            }
            Ok(IdempotencyDecision::Mismatch) => {
                return Err(poem::Error::from_string(
                    "idempotency-key reuse with a different request body",
                    StatusCode::CONFLICT,
                ));
            }
            Ok(IdempotencyDecision::Fresh) | Ok(IdempotencyDecision::NoKey) => {}
            Err(e) => {
                tracing::warn!(user_id = %user_id, error = %e, "idempotency check failed; proceeding without cache");
            }
        }
    }

    match executor.execute_image_generation(&request, &user_id).await {
        Ok(response) => {
            if let Some(ref key) = idem_key {
                let body = serde_json::to_vec(&response).unwrap_or_default();
                if !body.is_empty() {
                    if let Err(e) = idempotency::record(
                        pool,
                        &user_id,
                        key,
                        &request_hash,
                        200,
                        &body,
                        Some("application/json"),
                    )
                    .await
                    {
                        tracing::warn!(user_id = %user_id, error = %e, "idempotency record failed");
                    }
                }
            }
            Ok(Json(response))
        }
        Err(e) => {
            tracing::warn!(error = %e, "image generation failed");
            Err(mawi_core::error::into_poem_error(e))
        }
    }
}

/// Read + validate the optional `Idempotency-Key` header. Same shape
/// as the helper in `chat_new.rs` — extracted-or-not is a judgement
/// call: lifting to a shared helper requires a `Result<Option<String>,
/// poem::Error>` shape with a clear error mapping that's not currently
/// in `idempotency.rs`. Duplicate is small enough to live until a
/// third caller arrives.
fn parse_idempotency_header(req: &poem::Request) -> poem::Result<Option<String>> {
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
    idempotency::parse_key(s).map(Some).map_err(|e| {
        poem::Error::from_string(e.to_string(), StatusCode::BAD_REQUEST)
    })
}
