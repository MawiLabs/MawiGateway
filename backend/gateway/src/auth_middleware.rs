use poem::{Endpoint, Middleware, Request, Result, error::Error, http::StatusCode};
use sqlx::PgPool;
use core::auth::AuthService;
use moka::future::Cache;
use std::sync::OnceLock;
use std::time::Duration;

static SESSION_CACHE: OnceLock<Cache<String, mawi_core::auth::User>> = OnceLock::new();

pub struct AuthMiddleware;

impl<E: Endpoint> Middleware<E> for AuthMiddleware {
    type Output = AuthMiddlewareEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        AuthMiddlewareEndpoint { ep }
    }
}

pub struct AuthMiddlewareEndpoint<E> {
    ep: E,
}

impl<E: Endpoint> Endpoint for AuthMiddlewareEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        // skip auth for login/register/logout
        let path = req.uri().path();
        if path == "/v1/auth/login" || path == "/v1/auth/register" || path == "/v1/auth/logout" {
            return self.ep.call(req).await;
        }
        
        // extract token from cookie or Authorization header
        let session_token = req.headers()
            .get(poem::http::header::COOKIE)
            .and_then(|h| h.to_str().ok())
            .and_then(|cookie_str| {
                for cookie in cookie_str.split(';') {
                    let cookie = cookie.trim();
                    if let Some(value) = cookie.strip_prefix("session_token=") {
                        return Some(value.to_string());
                    }
                }
                None
            });

        let token = match session_token {
            Some(t) => t,
            None => {
                // Fallback: Check for Authorization: Bearer <token>
                req.headers()
                    .get(poem::http::header::AUTHORIZATION)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|auth_str| {
                        if auth_str.starts_with("Bearer ") {
                            Some(auth_str[7..].to_string())
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| Error::from_string("Missing session token or Authorization header", StatusCode::UNAUTHORIZED))?
            }
        };

        // validate token (needs DB pool)
        let pool = req.data::<PgPool>()
            .ok_or_else(|| Error::from_string("Database connection not available", StatusCode::INTERNAL_SERVER_ERROR))?;
            
        let auth_service = AuthService::new(pool.clone());
        
        // check session cache (60s TTL, was 300s but caused stale sessions)
        let cache = SESSION_CACHE.get_or_init(|| {
            Cache::builder()
                .time_to_live(Duration::from_secs(60)) // 60s TTL
                .max_capacity(10_000)
                .build()
        });

        if let Some(user) = cache.get(&token).await {
             enforce_rate_limit(&user.id)?;
             let mut req = req;
             req.extensions_mut().insert(user);
             return self.ep.call(req).await;
        }

        match auth_service.validate_session(&token).await {
            Ok(user) => {
                // Populate Cache
                cache.insert(token.clone(), user.clone()).await;
                enforce_rate_limit(&user.id)?;

                // attach user to request
                let mut req = req;
                req.extensions_mut().insert(user);
                self.ep.call(req).await
            }
            Err(_) => {
                Err(Error::from_string("Invalid or expired session", StatusCode::UNAUTHORIZED))
            }
        }
    }
}

/// Per-user rate-limit gate (#43). Runs after auth so denied requests
/// can't be triggered anonymously. Returns a 429 + `Retry-After` header
/// formatted Poem error on deny.
fn enforce_rate_limit(user_id: &str) -> Result<()> {
    use crate::rate_limit::{check_and_record, RateLimitDecision};

    match check_and_record(user_id) {
        RateLimitDecision::Allow => Ok(()),
        RateLimitDecision::Deny { retry_after_secs } => {
            let mut resp = poem::Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .header(poem::http::header::RETRY_AFTER, retry_after_secs.to_string())
                .header("X-RateLimit-Reset", retry_after_secs.to_string())
                .content_type("application/json")
                .body(format!(
                    "{{\"error\":\"rate_limited\",\"retry_after_secs\":{}}}",
                    retry_after_secs
                ));
            resp.set_status(StatusCode::TOO_MANY_REQUESTS);
            Err(Error::from_response(resp))
        }
    }
}
