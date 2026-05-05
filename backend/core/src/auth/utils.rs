use super::service::AuthService;
use poem::{error::Error, http::StatusCode, Request, Result as PoemResult};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use sqlx::Row;

/// Newtype wrapper for an authenticated request's scope list. Injected
/// into the request extensions alongside [`super::service::User`] so
/// handlers can check `is_satisfied_by(required, &scopes)` without
/// reaching back into the database. Session-cookie auth (browser login)
/// gets `["admin"]`; API-key auth gets the scopes column value from
/// the `api_keys` row.
#[derive(Debug, Clone)]
pub struct AuthScopes(pub Vec<String>);

/// Backwards-compat wrapper. New code should use
/// [`get_current_user_and_scopes`] so the scope list is available.
pub async fn get_current_user(req: &Request, pool: &PgPool) -> PoemResult<super::service::User> {
    let (user, _scopes) = get_current_user_and_scopes(req, pool).await?;
    Ok(user)
}

/// Resolve the authenticated user AND their scope list. Used by the
/// auth middleware so both can be injected into request extensions.
pub async fn get_current_user_and_scopes(
    req: &Request,
    pool: &PgPool,
) -> PoemResult<(super::service::User, Vec<String>)> {
    // 1. Try API Key (Bearer Token)
    if let Some(auth_header) = req.headers().get(poem::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(api_key) = auth_str.strip_prefix("Bearer ") {
                if api_key.starts_with("sk_") {
                    // It's an API Key. Validate it.
                    // Hash the secret
                    let mut hasher = Sha256::new();
                    hasher.update(api_key.as_bytes());
                    let key_hash = hex::encode(hasher.finalize());

                    let now = chrono::Utc::now().timestamp();

                    // Check DB. Pull `scopes` alongside the user_id so
                    // we can attach them to the request context — that's
                    // how downstream handlers know what the API key
                    // is allowed to do (#78).
                    let row = sqlx::query(
                        "SELECT user_id, expires_at, scopes FROM api_keys WHERE key_hash = $1",
                    )
                    .bind(&key_hash)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
                    })?;

                    if let Some(row) = row {
                        // Check expiration
                        let expires_at: Option<i64> = row.try_get("expires_at").ok();
                        if let Some(exp) = expires_at {
                            if now > exp {
                                return Err(Error::from_string(
                                    "API Key expired",
                                    StatusCode::UNAUTHORIZED,
                                ));
                            }
                        }

                        // Update last_used_at (async fire-and-forget)
                        let pool_clone = pool.clone();
                        let key_hash_clone = key_hash.clone();
                        tokio::spawn(async move {
                            if let Err(e) = sqlx::query(
                                "UPDATE api_keys SET last_used_at = $1 WHERE key_hash = $2",
                            )
                            .bind(now)
                            .bind(key_hash_clone)
                            .execute(&pool_clone)
                            .await
                            {
                                eprintln!("Failed to update api_key stats: {}", e);
                            }
                        });

                        let user_id: String = row.get("user_id");
                        // Pull scopes; default to ["admin"] if the
                        // column is unexpectedly null (shouldn't be —
                        // migration 034 made it NOT NULL DEFAULT
                        // '{admin}' — but defensive against legacy
                        // rows that predate the migration).
                        let scopes: Vec<String> = row
                            .try_get::<Vec<String>, _>("scopes")
                            .unwrap_or_else(|_| vec!["admin".to_string()]);

                        // Fetch full user
                        let auth_service = AuthService::new(pool.clone());
                        let user = auth_service
                            .get_user_by_id(&user_id)
                            .await
                            .map_err(|_| crate::api_error::poem_unauthorized("User not found"))?;

                        return Ok((user, scopes));
                    } else {
                        return Err(crate::api_error::poem_unauthorized("Invalid API Key"));
                    }
                }
            }
        }
    }

    // 2. Try Session Cookie (Fallback)
    let session_token = req
        .headers()
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
        })
        .ok_or_else(|| crate::api_error::poem_unauthorized(
            "Missing session token. Pass an API key as `Authorization: Bearer <key>` or sign in via /auth/login."
        ))?;

    // Validate session and get user
    let auth_service = AuthService::new(pool.clone());
    let user = auth_service
        .validate_session(&session_token)
        .await
        .map_err(|_| {
            crate::api_error::poem_unauthorized(
                "Invalid or expired session token. Sign in again or generate a fresh API key.",
            )
        })?;

    // Browser-cookie auth means the human owner of the account is
    // signed into the UI. They control the org top-to-bottom and get
    // the unrestricted "admin" scope. Per-scope keys are for the
    // programmatic case, where the operator deliberately limits them.
    Ok((user, vec!["admin".to_string()]))
}

pub fn get_session_token(req: &Request) -> Option<String> {
    let cookie_header = req
        .headers()
        .get(poem::http::header::COOKIE)?
        .to_str()
        .ok()?;
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix("session_token=") {
            return Some(value.to_string());
        }
    }
    None
}

pub fn set_session_cookie(response: &mut poem::Response, token: &str) {
    let cookie_value = format!(
        "session_token={}; Path=/; HttpOnly; Max-Age={}; SameSite=Lax",
        token,
        30 * 24 * 60 * 60
    );
    response.headers_mut().insert(
        poem::http::header::SET_COOKIE,
        cookie_value.parse().unwrap(),
    );
}
