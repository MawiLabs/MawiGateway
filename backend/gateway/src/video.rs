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

    eprintln!("🎬 Video generation request for service: {}", req.model);
    #[cfg(debug_assertions)]
    eprintln!("📝 Prompt: {}", req.prompt);
    #[cfg(not(debug_assertions))]
    eprintln!("📝 Prompt: [REDACTED]");

    let (chosen_model, mut response) = executor
        .execute_video_generation(&req.0, &user.id)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "video generation failed");
            mawi_core::error::into_poem_error(e)
        })?;

    // Encode the model that ACTUALLY handled the request (post-
    // failover) into the JOB_ID tag, NOT the service name. If we
    // failed over from Sora to Veo and tagged 'video-default', the
    // poll endpoint would resolve back to Sora (first model in the
    // pool) and 404 on a Veo job id every single time.
    if let Some(url) = &response.url {
        if url.starts_with("JOB_ID:") {
            response.url = Some(format!("{}|MODEL:{}", url, chosen_model.id));
        }
    }

    eprintln!(
        "🎬 Video job created on model '{}' (provider: {})",
        chosen_model.id, chosen_model.provider
    );

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

    // Always rewrite a non-empty video_url through the gateway's
    // /v1/videos/content/<id>/<model_id> proxy. Every video provider
    // returns a URL the browser can't load directly — Sora needs an
    // OpenAI Bearer header, Veo needs a Google API key as ?key=,
    // Runway needs a Bearer, ElevenLabs needs xi-api-key, even Pika
    // and Luma signed CDN URLs sometimes expire faster than the
    // canvas's load. Routing through the proxy makes "what URL do I
    // use?" deterministic — gateway always has the right adapter +
    // credentials, the <video> element only needs to know one path.
    if let Some(url) = status.get("video_url").and_then(|v| v.as_str()) {
        if !url.is_empty() && !url.starts_with("/v1/videos/content/") {
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
