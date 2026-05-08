use crate::executor::Executor;
use crate::idempotency::{self, IdempotencyDecision};
use mawi_core::types::{MusicGenerationRequest, TextToSpeechRequest};
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Json},
    Body, Response,
};
use sqlx::PgPool;
use std::sync::Arc;

#[handler]
pub async fn text_to_speech(
    req_http: &poem::Request,
    req: Json<TextToSpeechRequest>,
    executor: Data<&Arc<Executor>>,
    pool: Data<&PgPool>,
) -> poem::Result<Response> {
    // Extract user_id from session (injected by AuthMiddleware)
    let user = req_http
        .extensions()
        .get::<mawi_core::auth::User>()
        .ok_or_else(|| {
            poem::Error::from_string("Authentication required", StatusCode::UNAUTHORIZED)
        })?;
    let user_id = user.id.clone();

    eprintln!("🗣️ TTS Request for model: {}", req.model);

    // Idempotency-key handling — same pattern as chat / image (#41).
    // TTS responses are binary audio, so the cache stores raw bytes
    // plus the original `Content-Type` header so a replay re-emits
    // the exact provider output.
    let idem_key = idempotency::header_from_request(req_http)?;
    let body_bytes = serde_json::to_vec(&req.0).map_err(|e| {
        poem::Error::from_string(
            format!("could not re-serialise request for idempotency hash: {}", e),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    let request_hash = idempotency::hash_request("POST", "/v1/audio/speech", &body_bytes);

    if let Some(ref key) = idem_key {
        match idempotency::check(pool.0, &user_id, Some(key), &request_hash).await {
            Ok(IdempotencyDecision::Cached {
                response_body,
                content_type,
                ..
            }) => {
                let ct = content_type.unwrap_or_else(|| "audio/mpeg".to_string());
                return Ok(Response::builder()
                    .content_type(ct)
                    .body(Body::from(response_body)));
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

    let (content_type, bytes) = executor
        .execute_text_to_speech(&req.0, &user.id)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "TTS failed");
            mawi_core::error::into_poem_error(e)
        })?;

    if let Some(ref key) = idem_key {
        if let Err(e) = idempotency::record(
            pool.0,
            &user_id,
            key,
            &request_hash,
            200,
            &bytes,
            Some(&content_type),
        )
        .await
        {
            tracing::warn!(user_id = %user_id, error = %e, "idempotency record failed");
        }
    }

    Ok(Response::builder()
        .content_type(content_type)
        .body(Body::from(bytes)))
}

/// Music generation handler — `POST /v1/audio/music`.
/// Mirrors the TTS handler's contract (auth, idempotency, binary
/// audio response) but takes a MusicGenerationRequest and routes to
/// `executor.execute_music_generation`. Different endpoint from
/// `/v1/audio/speech` because the upstream provider APIs diverge:
/// ElevenLabs Music posts to `/v1/music` with prompt + length while
/// TTS posts to `/v1/text-to-speech/{voice_id}` with text + voice.
#[handler]
pub async fn generate_music(
    req_http: &poem::Request,
    req: Json<MusicGenerationRequest>,
    executor: Data<&Arc<Executor>>,
    pool: Data<&PgPool>,
) -> poem::Result<Response> {
    let user = req_http
        .extensions()
        .get::<mawi_core::auth::User>()
        .ok_or_else(|| {
            poem::Error::from_string("Authentication required", StatusCode::UNAUTHORIZED)
        })?;
    let user_id = user.id.clone();

    eprintln!("🎵 Music Request for model: {}", req.model);

    let idem_key = idempotency::header_from_request(req_http)?;
    let body_bytes = serde_json::to_vec(&req.0).map_err(|e| {
        poem::Error::from_string(
            format!("could not re-serialise request for idempotency hash: {}", e),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    let request_hash = idempotency::hash_request("POST", "/v1/audio/music", &body_bytes);

    if let Some(ref key) = idem_key {
        match idempotency::check(pool.0, &user_id, Some(key), &request_hash).await {
            Ok(IdempotencyDecision::Cached {
                response_body,
                content_type,
                ..
            }) => {
                let ct = content_type.unwrap_or_else(|| "audio/mpeg".to_string());
                return Ok(Response::builder()
                    .content_type(ct)
                    .body(Body::from(response_body)));
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

    let (content_type, bytes) = executor
        .execute_music_generation(&req.0, &user.id)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "music generation failed");
            mawi_core::error::into_poem_error(e)
        })?;

    if let Some(ref key) = idem_key {
        if let Err(e) = idempotency::record(
            pool.0,
            &user_id,
            key,
            &request_hash,
            200,
            &bytes,
            Some(&content_type),
        )
        .await
        {
            tracing::warn!(user_id = %user_id, error = %e, "idempotency record failed");
        }
    }

    Ok(Response::builder()
        .content_type(content_type)
        .body(Body::from(bytes)))
}
