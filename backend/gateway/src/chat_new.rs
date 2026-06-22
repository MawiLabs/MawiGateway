use crate::executor::Executor;
use crate::idempotency::{self, IdempotencyDecision};
use crate::semantic_cache::{self, CacheConfig, CacheDecision};
use futures::StreamExt;
use mawi_core::api_error::{error_type, OpenAiError, OpenAiErrorResponse};
use mawi_core::unified::{UnifiedChatRequest, UnifiedChatResponse};
use poem::{web::Data, Body, Request};
use poem_openapi::{
    payload::{Binary, Json},
    ApiResponse, OpenApi,
};
use std::sync::Arc;

/// Response variants for `POST /v1/chat/completions`. Error variants
/// carry an [`OpenAiErrorResponse`] body so the OpenAI / Anthropic
/// SDKs unmarshal it into their typed exception classes (#84). The
/// streaming variant keeps its bare `Binary<Body>` because SSE error
/// frames are emitted inline as `data: {...}\n\n` events, not as the
/// HTTP response body.
#[derive(ApiResponse)]
enum ChatResponse {
    #[oai(status = 200)]
    Ok(Json<UnifiedChatResponse>),
    #[oai(status = 200, content_type = "text/event-stream")]
    Streaming(Binary<Body>),
    #[oai(status = 400)]
    BadRequest(Json<OpenAiErrorResponse>),
    #[oai(status = 401)]
    Unauthorized(Json<OpenAiErrorResponse>),
    /// 403 — authenticated but the API key's scopes don't permit this
    /// call (e.g. a `chat:other-service` key calling a different
    /// service). Distinct from 401 so SDKs raise PermissionDeniedError
    /// rather than AuthenticationError.
    #[oai(status = 403)]
    Forbidden(Json<OpenAiErrorResponse>),
    #[oai(status = 409)]
    IdempotencyMismatch(Json<OpenAiErrorResponse>),
    /// 429 — at least one upstream provider rate-limited us. Body
    /// carries the upstream message (e.g. "insufficient_quota") so the
    /// caller can tell whether to back off or top up billing.
    #[oai(status = 429)]
    TooManyRequests(Json<OpenAiErrorResponse>),
    /// 503 — every healthy model in the pool is currently unavailable
    /// (5xx / connection errors / circuit-breaker tripped). Caller
    /// should retry after a delay.
    #[oai(status = 503)]
    ServiceUnavailable(Json<OpenAiErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<OpenAiErrorResponse>),
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
        // `service` is the routing primitive — that's the whole point
        // of MawiGateway. We deliberately do NOT auto-fall back to
        // `model` because that would let clients address models
        // directly and bypass services (pools, strategies, planners,
        // budgets, caching). Reject with a clear message.
        if request.service.is_empty() {
            return ChatResponse::BadRequest(Json(OpenAiError::for_param(
                "missing required field: 'service'. MawiGateway routes through \
                 services, not models — create a service first (UI: /services, \
                 CLI: `mawi services create`).",
                error_type::INVALID_REQUEST,
                "service",
            )));
        }

        // Extract user_id (injected by AuthMiddleware)
        let user = match req.extensions().get::<mawi_core::auth::User>() {
            Some(u) => u,
            None => {
                return ChatResponse::Unauthorized(Json(OpenAiError::with_code(
                    "Authentication required: pass an API key via the \
                     Authorization: Bearer header.",
                    error_type::AUTHENTICATION,
                    "missing_credentials",
                )))
            }
        };
        let user_id = user.id.clone();

        // Authorization gate: this endpoint requires either the
        // `chat` scope (any service) or `chat:<service>` (just this
        // one) — `admin` implies both per the rules in
        // mawi_core::scopes (#78). Browser-cookie auth gets `admin`
        // automatically; per-scope API keys are gated here.
        let granted = req
            .extensions()
            .get::<mawi_core::auth::utils::AuthScopes>()
            .map(|s| s.0.clone())
            .unwrap_or_default();
        let required = format!("chat:{}", request.service);
        if !mawi_core::scopes::is_satisfied_by(&required, &granted)
            && !mawi_core::scopes::is_satisfied_by(mawi_core::scopes::CHAT, &granted)
        {
            return ChatResponse::Forbidden(Json(OpenAiError::with_code(
                format!(
                    "API key lacks scope to call chat completions on service '{}'. \
                     Required: 'chat' or 'chat:{}'. Granted: {:?}.",
                    request.service, request.service, granted
                ),
                error_type::PERMISSION,
                "insufficient_scope",
            )));
        }

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
                    // Mid-stream errors emit the OpenAI-shape envelope
                    // (#84) wrapped in an SSE event. Streaming SDKs that
                    // parse `data: {error: {message, type, code}}` get
                    // the same structured info as non-streaming clients.
                    let envelope = OpenAiError::new(e.to_string(), error_type::API);
                    let json = serde_json::to_string(&envelope).unwrap_or_default();
                    let sse_msg = format!("data: {}\n\n", json);
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
            Err(e) => {
                return ChatResponse::BadRequest(Json(OpenAiError::with_code(
                    e.to_string(),
                    error_type::INVALID_REQUEST,
                    "invalid_idempotency_key",
                )))
            }
        };

        // Stable request hash: re-serialise the parsed struct to JSON.
        // serde produces the same byte sequence for the same struct so
        // a retry with logically identical input gets the same hash.
        let body_bytes = match serde_json::to_vec(&request) {
            Ok(b) => b,
            Err(e) => {
                return ChatResponse::InternalError(Json(OpenAiError::new(
                    format!("could not re-serialise request for idempotency hash: {}", e),
                    error_type::API,
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
                    return ChatResponse::IdempotencyMismatch(Json(OpenAiError::with_code(
                        "Idempotency-Key reused with a different request body. \
                         Either reuse the same body or pick a fresh key.",
                        error_type::CONFLICT,
                        "idempotency_mismatch",
                    )));
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

        // ---- Semantic cache lookup (Tier-2 #6) -----------------------------
        // Layered after idempotency so an exact replay still hits the
        // microsecond-fast idempotency path; the semantic cache catches
        // semantically equivalent prompts that have a different
        // idempotency key (or none at all).
        //
        // Cache misses or a disabled service short-circuit to the
        // executor below — there's no "soft fail closed" path that
        // could lose user requests because the cache is unhappy.
        let cache_cfg = CacheConfig::load(pool.0, &request.service)
            .await
            .unwrap_or(CacheConfig {
                enabled: false,
                similarity_threshold: 0.95,
                ttl_seconds: 3600,
            });

        if cache_cfg.enabled {
            if let CacheDecision::Hit {
                response,
                similarity,
                exact,
            } = semantic_cache::lookup(
                pool.0,
                &user_id,
                &request.service,
                &request_hash,
                &request.messages,
                &cache_cfg,
            )
            .await
            {
                tracing::info!(
                    service = %request.service,
                    user_id = %user_id,
                    similarity = similarity,
                    exact = exact,
                    "semantic cache HIT"
                );
                return ChatResponse::Ok(Json(response));
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

                // Best-effort store in the semantic cache. Errors are
                // already logged inside `store`; there's no recovery to
                // do here — the user has their response.
                if cache_cfg.enabled {
                    semantic_cache::store(
                        pool.0,
                        &user_id,
                        &request.service,
                        &request_hash,
                        &request.messages,
                        &response,
                        &cache_cfg,
                    )
                    .await;
                }

                ChatResponse::Ok(Json(response))
            }
            Err(e) => {
                // Errors are NOT recorded under the idempotency key so a
                // client retry gets a fresh attempt. (Stripe records 4xx
                // but not 5xx; we keep it simple — no recording for any
                // error path until a clear use case demands otherwise.)
                eprintln!("Chat execution failed: {}", e);
                // Try to recover the typed ProviderError that bubbled up
                // from the failover loop. When every model in the pool
                // hit the same upstream failure (e.g. all OpenAI keys
                // out of quota), this lets the response carry the right
                // 4xx (rate_limit / unauthorized / bad_request) instead
                // of a generic 500 — clients can react meaningfully and
                // the user sees "your OpenAI quota ran out" instead of
                // "internal server error".
                if let Some(pe) = mawi_core::error::downcast(&e) {
                    use mawi_core::error::ProviderError;
                    let body = OpenAiError::new(pe.message().to_string(), error_type::API);
                    return match pe {
                        ProviderError::RateLimit { .. } => ChatResponse::TooManyRequests(Json(body)),
                        ProviderError::Unauthorized { .. } => ChatResponse::Unauthorized(Json(body)),
                        ProviderError::BadRequest { .. } => ChatResponse::BadRequest(Json(body)),
                        ProviderError::Unavailable { .. } => ChatResponse::ServiceUnavailable(Json(body)),
                        ProviderError::Timeout { .. } => ChatResponse::ServiceUnavailable(Json(body)),
                        _ => ChatResponse::InternalError(Json(body)),
                    };
                }
                ChatResponse::InternalError(Json(OpenAiError::new(
                    format!("Request failed: {}", e),
                    error_type::API,
                )))
            }
        }
    }
}

#[derive(poem_openapi::Tags)]
enum ApiTags {
    Chat,
}
