use crate::executor::Executor;
use crate::idempotency::{self, IdempotencyDecision};
use mawi_core::types::SpeechToSpeechRequest;
use poem::http::StatusCode;
use poem::{
    handler,
    web::{Data, Multipart},
    Body, Response,
};
use sqlx::PgPool;
use std::sync::Arc;

#[handler]
pub async fn speech_to_speech_endpoint(
    req: &poem::Request,
    mut multipart: Multipart,
    executor: Data<&Arc<Executor>>,
    pool: Data<&PgPool>,
) -> poem::Result<Response> {
    // 1. Authenticate. The previous implementation skipped this entirely
    //    — the AuthMiddleware still ran, so the session was validated,
    //    but the handler never extracted the user, meaning audit logs
    //    couldn't attribute calls to a specific account. Fixed alongside
    //    the idempotency-key wiring (#41).
    let user = req
        .extensions()
        .get::<mawi_core::auth::User>()
        .ok_or_else(|| {
            poem::Error::from_string("Authentication required", StatusCode::UNAUTHORIZED)
        })?;
    let user_id = user.id.clone();

    // 2. Drain multipart into owned fields. Keep the raw audio bytes so
    //    we can hash the request and pass them to the executor.
    let mut audio_data: Option<Vec<u8>> = None;
    let mut model: Option<String> = None;
    let mut voice: Option<String> = None;

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
            Some("voice") => {
                voice = field.text().await.ok();
            }
            _ => {}
        }
    }

    let audio_data = audio_data
        .ok_or_else(|| poem::Error::from_string("Missing 'file' field", StatusCode::BAD_REQUEST))?;
    let model = model.ok_or_else(|| {
        poem::Error::from_string("Missing 'model' field", StatusCode::BAD_REQUEST)
    })?;

    eprintln!(
        "🔄 STS Request for model: {} ({} bytes) by user {}",
        model,
        audio_data.len(),
        user_id
    );

    // 3. Idempotency hash — same multipart pattern as STT.
    let idem_key = idempotency::header_from_request(req)?;
    let mut body_buf: Vec<u8> = Vec::with_capacity(audio_data.len() + 128);
    body_buf.extend_from_slice(b"model=");
    body_buf.extend_from_slice(model.as_bytes());
    body_buf.push(b'\n');
    body_buf.extend_from_slice(b"voice=");
    body_buf.extend_from_slice(voice.as_deref().unwrap_or("").as_bytes());
    body_buf.push(b'\n');
    body_buf.extend_from_slice(b"file=");
    body_buf.extend_from_slice(&audio_data);
    let request_hash = idempotency::hash_request("POST", "/v1/audio/speech-to-speech", &body_buf);

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

    let request = SpeechToSpeechRequest { model, voice };

    let result_audio = executor
        .execute_speech_to_speech(&audio_data, &request)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "speech-to-speech failed");
            mawi_core::error::into_poem_error(e)
        })?;

    // S2S provider returns mpeg today. If a future provider returns
    // a different container, the executor signature should grow a
    // content_type return like TTS does.
    let response_content_type = "audio/mpeg";
    if let Some(ref key) = idem_key {
        if let Err(e) = idempotency::record(
            pool.0,
            &user_id,
            key,
            &request_hash,
            200,
            &result_audio,
            Some(response_content_type),
        )
        .await
        {
            tracing::warn!(user_id = %user_id, error = %e, "idempotency record failed");
        }
    }

    Ok(Response::builder()
        .content_type(response_content_type)
        .body(Body::from(result_audio)))
}
