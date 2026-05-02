use crate::executor::Executor;
use mawi_core::types::{VideoGenerationRequest, VideoGenerationResponse};
use poem::web::Data;
use poem::{handler, web::Json};
use std::sync::Arc;

#[handler]
pub async fn generate_video(
    req_http: &poem::Request,
    req: Json<VideoGenerationRequest>,
    executor: Data<&Arc<Executor>>,
) -> poem::Result<Json<VideoGenerationResponse>> {
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

    eprintln!("🎬 Video generation request for model: {}", req.model);
    #[cfg(debug_assertions)]
    eprintln!("📝 Prompt: {}", req.prompt);
    #[cfg(not(debug_assertions))]
    eprintln!("📝 Prompt: [REDACTED]");

    let mut response: VideoGenerationResponse = executor
        .execute_video_generation(&req.0, &user.id)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "video generation failed");
            mawi_core::error::into_poem_error(e)
        })?;

    // Append model ID to job ID for frontend polling
    if let Some(url) = &response.url {
        if url.starts_with("JOB_ID:") {
            response.url = Some(format!("{}|MODEL:{}", url, req.model));
        }
    }

    Ok(Json(response))
}

#[handler]
pub async fn poll_video_job(
    poem::web::Path((job_id, model_id)): poem::web::Path<(String, String)>,
    executor: Data<&Arc<Executor>>,
) -> poem::Result<poem::web::Json<serde_json::Value>> {
    let mut status = executor
        .poll_video_job(&job_id, &model_id)
        .await
        .map_err(|e| {
            tracing::debug!(error = %e, job_id = %job_id, "poll_video_job failed");
            mawi_core::error::into_poem_error(e)
        })?;

    // OpenAI Sora returns a video_url like https://api.openai.com/v1/
    // videos/<id> that requires the upstream API key on every byte
    // request — the browser can't reach it directly. Rewrite to the
    // gateway's own /v1/videos/content/<id>/<model_id> proxy so the
    // <video src=…> element on the canvas can fetch through the
    // session it already holds.
    if let Some(url) = status.get("video_url").and_then(|v| v.as_str()) {
        if url.starts_with("https://api.openai.com/")
            || url.contains("api.dev.runwayml.com")
            || url.contains("api.elevenlabs.io")
        {
            let proxied = format!("/v1/videos/content/{}/{}", job_id, model_id);
            status["video_url"] = serde_json::Value::String(proxied);
        }
    }

    Ok(poem::web::Json(status))
}

#[handler]
pub async fn proxy_video_content(
    poem::web::Path((generation_id, model_id)): poem::web::Path<(String, String)>,
    executor: Data<&Arc<Executor>>,
) -> poem::Result<poem::Response> {
    // Get video content from provider with authentication
    let video_data = executor
        .get_video_content(&generation_id, &model_id)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, generation_id = %generation_id, "video content fetch failed");
            mawi_core::error::into_poem_error(e)
        })?;

    Ok(poem::Response::builder()
        .content_type("video/mp4")
        .body(video_data))
}
