use crate::executor::Executor;
use crate::idempotency::{self, IdempotencyDecision};
use mawi_core::types::{AudioTranscriptionRequest, AudioTranscriptionResponse};
use poem::http::StatusCode;
use poem::web::Json;
use poem::{
    handler,
    web::{Data, Multipart},
};
use sqlx::PgPool;
use std::sync::Arc;

#[handler]
pub async fn transcribe_audio(
    req: &poem::Request,
    mut multipart: Multipart,
    executor: Data<&Arc<Executor>>,
    pool: Data<&PgPool>,
) -> poem::Result<Json<AudioTranscriptionResponse>> {
    // 1. Authenticate
    let user = req
        .extensions()
        .get::<mawi_core::auth::User>()
        .ok_or_else(|| {
            poem::Error::from_string("Authentication required", StatusCode::UNAUTHORIZED)
        })?;
    let user_id = user.id.clone();

    // 2. Parse multipart fields once into owned values; we need them
    //    available BOTH for the idempotency hash and for the executor
    //    call below.
    let mut audio_data: Option<Vec<u8>> = None;
    let mut model: Option<String> = None;
    let mut language: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().map(|s| s.to_string());
        match name.as_deref() {
            Some("file") => {
                let bytes = field.bytes().await.map_err(|e| {
                    poem::Error::from_string(
                        format!("Failed to read audio file: {}", e),
                        StatusCode::BAD_REQUEST,
                    )
                })?;
                audio_data = Some(bytes.to_vec());
            }
            Some("model") => {
                let text = field.text().await.map_err(|e| {
                    poem::Error::from_string(
                        format!("Failed to read model: {}", e),
                        StatusCode::BAD_REQUEST,
                    )
                })?;
                model = Some(text);
            }
            Some("language") => {
                language = field.text().await.ok();
            }
            _ => {}
        }
    }

    let audio_data = audio_data.ok_or_else(|| {
        poem::Error::from_string("Missing 'file' field", StatusCode::BAD_REQUEST)
    })?;
    let model = model.ok_or_else(|| {
        poem::Error::from_string("Missing 'model' field", StatusCode::BAD_REQUEST)
    })?;

    eprintln!(
        "🎤 STT Request for model: {} ({} bytes) by user {}",
        model,
        audio_data.len(),
        user_id
    );

    let request_obj = AudioTranscriptionRequest {
        model: model.clone(),
        language: language.clone(),
    };

    // 3. Idempotency hash. Multipart needs a deterministic
    //    serialization — we hash a stable concatenation of the
    //    text fields plus the raw file bytes. Any change to file
    //    content or params produces a new hash.
    let idem_key = idempotency::header_from_request(req)?;
    let mut body_buf: Vec<u8> = Vec::with_capacity(audio_data.len() + 128);
    body_buf.extend_from_slice(b"model=");
    body_buf.extend_from_slice(model.as_bytes());
    body_buf.push(b'\n');
    body_buf.extend_from_slice(b"language=");
    body_buf.extend_from_slice(language.as_deref().unwrap_or("").as_bytes());
    body_buf.push(b'\n');
    body_buf.extend_from_slice(b"file=");
    body_buf.extend_from_slice(&audio_data);
    let request_hash = idempotency::hash_request("POST", "/v1/audio/transcriptions", &body_buf);

    if let Some(ref key) = idem_key {
        match idempotency::check(pool.0, &user_id, Some(key), &request_hash).await {
            Ok(IdempotencyDecision::Cached { response_body, .. }) => {
                if let Ok(resp) =
                    serde_json::from_slice::<AudioTranscriptionResponse>(&response_body)
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

    let text = executor
        .execute_transcription(&audio_data, &request_obj, &user_id)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "transcription failed");
            mawi_core::error::into_poem_error(e)
        })?;

    let response = AudioTranscriptionResponse { text };

    if let Some(ref key) = idem_key {
        let body = serde_json::to_vec(&response).unwrap_or_default();
        if !body.is_empty() {
            if let Err(e) = idempotency::record(
                pool.0,
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
