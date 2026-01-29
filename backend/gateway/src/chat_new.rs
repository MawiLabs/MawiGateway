use poem_openapi::{
    payload::{Json, Binary},
    OpenApi, ApiResponse,
};
use poem::{web::Data, Request, Body};
use mawi_core::unified::{UnifiedChatRequest, UnifiedChatResponse};
use std::sync::Arc;
use crate::executor::Executor;
use futures::StreamExt;

#[derive(ApiResponse)]
enum ChatResponse {
    #[oai(status = 200)]
    Ok(Json<UnifiedChatResponse>),
    #[oai(status = 200, content_type = "text/event-stream")]
    Streaming(Binary<Body>),
    #[oai(status = 401)]
    Unauthorized(Json<String>),
    #[oai(status = 500)]
    InternalError(Json<String>),
}

pub struct ChatApi {
    pub executor: Arc<Executor>,
}

#[OpenApi]
impl ChatApi {
    /// Create chat completion
    #[oai(path = "/chat/completions", method = "post", tag = "ApiTags::Chat")]
    async fn chat_completions(
        &self,
        pool: Data<&sqlx::PgPool>,
        req: &Request,
        Json(request): Json<UnifiedChatRequest>,
    ) -> ChatResponse {
        
        // Extract user_id (injected by AuthMiddleware)
        let user = match req.extensions().get::<mawi_core::auth::User>() {
             Some(u) => u,
             None => return ChatResponse::Unauthorized(Json("Authentication required".to_string())),
        };
        let user_id = user.id.clone();

        // Streaming Path
        if request.stream.unwrap_or(false) {
            let executor = self.executor.clone();
            let stream = executor.execute_chat_stream(request, &user_id);
            
            let sse_stream = stream.map(|result| {
                match result {
                    Ok(event) => {
                        let json = serde_json::to_string(&event).unwrap_or_default();
                        let sse_msg = format!("data: {}\n\n", json);
                        Ok::<Vec<u8>, std::io::Error>(sse_msg.into_bytes())
                    },
                    Err(e) => {
                         let error_json = serde_json::json!({
                            "type": "error",
                            "data": e.to_string()
                        }).to_string();
                        let sse_msg = format!("data: {}\n\n", error_json);
                        Ok(sse_msg.into_bytes())
                    }
                }
            });

            return ChatResponse::Streaming(Binary(Body::from_bytes_stream(sse_stream)));
        }

        // Sync Path
        match self.executor.execute_chat(&request, &user_id).await {
            Ok(response) => ChatResponse::Ok(Json(response)),
            Err(e) => {
                eprintln!("Chat execution failed: {}", e);
                ChatResponse::InternalError(Json(format!("Request failed: {}", e)))
            }
        }
    }
}

#[derive(poem_openapi::Tags)]
enum ApiTags {
    Chat,
}
