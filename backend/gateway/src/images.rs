use crate::executor::Executor;
use mawi_core::types::{ImageGenerationRequest, ImageGenerationResponse};
use poem::{
    handler,
    web::{Data, Json},
};
use std::sync::Arc;

#[handler]
pub async fn image_generations(
    req: &poem::Request,
    Data(executor): Data<&Arc<Executor>>,
    Json(request): Json<ImageGenerationRequest>,
) -> poem::Result<Json<ImageGenerationResponse>> {
    // Extract user_id from session (injected by AuthMiddleware)
    let user = req
        .extensions()
        .get::<mawi_core::auth::User>()
        .ok_or_else(|| {
            poem::Error::from_string(
                "Authentication required",
                poem::http::StatusCode::UNAUTHORIZED,
            )
        })?;

    match executor.execute_image_generation(&request, &user.id).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::warn!(error = %e, "image generation failed");
            Err(mawi_core::error::into_poem_error(e))
        }
    }
}
