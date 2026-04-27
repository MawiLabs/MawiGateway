use crate::executor::Executor;
use crate::idempotency::{self, IdempotencyDecision};
use futures::StreamExt;
use mawi_core::unified::{UnifiedChatRequest, UnifiedChatResponse};
use poem::{web::Data, Body, Request};
use poem_openapi::{
    payload::{Binary, Json},
    ApiResponse, OpenApi,
};
use std::sync::Arc;

#[derive(ApiResponse)]
enum ChatResponse {
    #[oai(status = 200)]
    Ok(Json<UnifiedChatResponse>),
    #[oai(status = 200, content_type = "text/event-stream")]
    Streaming(Binary<Body>),
    #[oai(status = 400)]
    BadRequest(Json<String>),
    #[oai(status = 401)]
    Unauthorized(Json<String>),
    #[oai(status = 409)]
    IdempotencyMismatch(Json<String>),
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

        // Streaming Path — idempotency keys are not honoured here
        // (#41): the response is incremental and can't be cached
        // mid-stream. Streaming clients are responsible for their
        // own retry semantics (already partly addressed by #30).
        if request.stream.unwrap_or(false) {
            let executor = self.executor.clone();
            let stream = executor.execute_chat_stream(request, &user_id);

            let sse_stream = stream.map(|result| match result {
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
            });

            return ChatResponse::Streaming(Binary(Body::from_bytes_stream(sse_stream)));
        }

        // Sync path — idempotency-key support (#41).
        //
        // The header is optional. If present:
        //   - validate format (8..256 chars, [A-Za-z0-9_-.])
        //   - hash the request body
        //   - check the cache: cached → return; mismatch → 409; fresh → proceed
        //   - on success, record the response so a retry returns it instead
        //     of executing again (and re-billing the provider).
        let idem_key = match idempotency::header_from_request(req) {
            Ok(k) => k,
            Err(e) => return ChatResponse::BadRequest(Json(e.to_string())),
        };

        // Stable request hash: re-serialise the parsed struct to JSON.
        // serde produces the same byte sequence for the same struct so
        // a retry with logically identical input gets the same hash.
        let body_bytes = match serde_json::to_vec(&request) {
            Ok(b) => b,
            Err(e) => {
                return ChatResponse::InternalError(Json(format!(
                    "could not re-serialise request for idempotency hash: {}",
                    e
                )))
            }
        };
        let request_hash = idempotency::hash_request("POST", "/v1/chat/completions", &body_bytes);

        if let Some(ref key) = idem_key {
            match idempotency::check(pool.0, &user_id, Some(key), &request_hash).await {
                Ok(IdempotencyDecision::Cached { response_body, .. }) => {
                    match serde_json::from_slice::<UnifiedChatResponse>(&response_body) {
                        Ok(resp) => return ChatResponse::Ok(Json(resp)),
                        Err(e) => {
                            // Stored row exists but won't deserialise — should
                            // never happen unless the schema changed under us.
                            // Fall through to a fresh execution rather than
                            // returning a corrupt response.
                            tracing::warn!(
                                user_id = %user_id,
                                error = %e,
                                "idempotency cache hit but stored body did not deserialise; re-executing"
                            );
                        }
                    }
                }
                Ok(IdempotencyDecision::Mismatch) => {
                    return ChatResponse::IdempotencyMismatch(Json(
                        "idempotency-key reuse with a different request body".to_string(),
                    ));
                }
                Ok(IdempotencyDecision::Fresh) | Ok(IdempotencyDecision::NoKey) => {}
                Err(e) => {
                    // Treat the cache as unavailable and proceed — better
                    // to re-execute than to fail the request because of
                    // an idempotency-table problem. Logged so SREs see it.
                    tracing::warn!(
                        user_id = %user_id,
                        error = %e,
                        "idempotency check failed; proceeding without cache"
                    );
                }
            }
        }

        match self.executor.execute_chat(&request, &user_id).await {
            Ok(response) => {
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
                            tracing::warn!(
                                user_id = %user_id,
                                error = %e,
                                "idempotency record failed; response served but a retry will re-execute"
                            );
                        }
                    }
                }
                ChatResponse::Ok(Json(response))
            }
            Err(e) => {
                // Errors are NOT recorded under the idempotency key so a
                // client retry gets a fresh attempt. (Stripe records 4xx
                // but not 5xx; we keep it simple — no recording for any
                // error path until a clear use case demands otherwise.)
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
