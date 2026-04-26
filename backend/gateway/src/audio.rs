use crate::executor::Executor;
use mawi_core::types::TextToSpeechRequest;
use poem::{
    handler,
    web::{Data, Json},
    Body, Response,
};
use std::sync::Arc;

#[handler]
pub async fn text_to_speech(
    req_http: &poem::Request,
    req: Json<TextToSpeechRequest>,
    executor: Data<&Arc<Executor>>,
) -> poem::Result<Response> {
    // Extract user_id from session (injected by AuthMiddleware)
    let user = req_http
        .extensions()
        .get::<mawi_core::auth::User>()
        .ok_or_else(|| {
            poem::Error::from_string(
                "Authentication required",
                poem::http::StatusCode::UNAUTHORIZED,
            )
        })?;

    eprintln!("🗣️ TTS Request for model: {}", req.model);

    let (content_type, bytes) = executor
        .execute_text_to_speech(&req.0, &user.id)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "TTS failed");
            mawi_core::error::into_poem_error(e)
        })?;

    Ok(Response::builder()
        .content_type(content_type)
        .body(Body::from(bytes)))
}
