use poem::{handler, web::{Json, Data}};
use mawi_core::types::{ImageGenerationRequest, ImageGenerationResponse};
use std::sync::Arc;
use crate::executor::Executor;

#[handler]
pub async fn image_generations(
    req: &poem::Request,
    Data(executor): Data<&Arc<Executor>>,
    Json(request): Json<ImageGenerationRequest>,
) -> poem::Result<Json<ImageGenerationResponse>> {
    // Extract user_id from session (injected by AuthMiddleware)
    let user = req.extensions().get::<mawi_core::auth::User>()
        .ok_or_else(|| poem::Error::from_string("Authentication required", poem::http::StatusCode::UNAUTHORIZED))?;

    match executor.execute_image_generation(&request, &user.id).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            eprintln!("Image generation failed: {}", e);
            Err(poem::Error::from_string(
                format!("Request failed: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}
