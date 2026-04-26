use crate::executor::Executor;
use futures::stream::AbortHandle;
use futures::StreamExt;
use mawi_core::unified::{UnifiedChatRequest, UnifiedChatResponse};
use poem::{web::Data, Body, Request};
use poem_openapi::{
    payload::{Binary, Json},
    ApiResponse, OpenApi,
};
use std::sync::Arc;

/// RAII guard: when this is dropped, it fires `abort()` on the contained
/// `AbortHandle`. Used to wire "the response body got dropped" (which
/// happens immediately when the client disconnects) to "cancel the
/// upstream provider stream and free its resources".
struct AbortOnDrop(AbortHandle);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
        tracing::debug!("chat stream aborted (client disconnected or request finished)");
    }
}

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
        _pool: Data<&sqlx::PgPool>,
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
            let inner_stream = executor.execute_chat_stream(request, &user_id);

            // Wrap the upstream provider stream with `abortable` so that
            // when the client disconnects (the response future is dropped
            // by Poem), our `AbortOnDrop` guard fires `abort()`. That
            // cancels the inner stream's future — which in turn cancels
            // the in-flight reqwest call to the provider, freeing both
            // our connection slot and the provider's quota immediately.
            //
            // Without this: the user closes the tab → we keep streaming
            // tokens from OpenAI until completion, charging quota for
            // bytes nobody will ever see. See #30.
            let (abortable_stream, abort_handle) =
                futures::stream::abortable(inner_stream);
            let _abort_guard = AbortOnDrop(abort_handle);

            let sse_stream = abortable_stream
                .map(|result| match result {
                    Ok(event) => {
                        let json = serde_json::to_string(&event).unwrap_or_default();
                        let sse_msg = format!("data: {}\n\n", json);
                        Ok::<Vec<u8>, std::io::Error>(sse_msg.into_bytes())
                    }
                    Err(e) => {
                        let error_json = serde_json::json!({
                            "type": "error",
                            "data": e.to_string()
                        })
                        .to_string();
                        let sse_msg = format!("data: {}\n\n", error_json);
                        Ok(sse_msg.into_bytes())
                    }
                })
                // Move the guard into the stream so it stays alive for the
                // stream's lifetime (and gets dropped exactly when the
                // response body is dropped).
                .chain(futures::stream::once(async move {
                    drop(_abort_guard);
                    Ok::<Vec<u8>, std::io::Error>(Vec::new())
                }));

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
