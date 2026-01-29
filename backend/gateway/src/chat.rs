use poem::{handler, web::{Json, Data}, IntoResponse};
use mawi_core::unified::UnifiedChatRequest;
use std::sync::Arc;
use crate::executor::Executor;
use futures::StreamExt;

/// Unified chat completions endpoint
/// Routes requests through services with weighted distribution and automatic failover
/// Supports SSE streaming for Agentic Services
#[handler]
pub async fn chat_completions(
    Data(executor): Data<&Arc<Executor>>,
    Data(pool): Data<&sqlx::PgPool>,
    req: &poem::Request,
    Json(request): Json<UnifiedChatRequest>,
) -> poem::Result<poem::Response> {
    
    // Extract user_id from session (injected by AuthMiddleware - Critical)
    let user = req.extensions().get::<mawi_core::auth::User>()
        .ok_or_else(|| poem::Error::from_string("Authentication required", poem::http::StatusCode::UNAUTHORIZED))?;
    let user_id = user.id.clone();

    
    // Check for streaming request
    if request.stream.unwrap_or(false) {
        let stream = executor.execute_chat_stream(request, &user_id);
        
        let sse_stream = stream.map(|result| {
            match result {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap_or_default();
                    poem::web::sse::Event::message(json)
                },
                Err(e) => {
                    let error_json = serde_json::json!({
                        "type": "error",
                        "data": e.to_string()
                    }).to_string();
                    poem::web::sse::Event::message(error_json)
                }
            }
        });

        // Use poem's SSE response builder with keep-alive to force immediate flush/connection
        return Ok(poem::web::sse::SSE::new(sse_stream)
            .keep_alive(std::time::Duration::from_secs(1))
            .into_response());
    }

    // Standard synchronous execution
    match executor.execute_chat(&request, &user_id).await {
        Ok(response) => Ok(Json(response).into_response()),
        Err(e) => {
            eprintln!("Chat execution failed: {}", e);
            Err(poem::Error::from_string(
                format!("Request failed: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}
