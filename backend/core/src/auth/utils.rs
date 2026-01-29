use poem::{Request, Result as PoemResult, error::Error, http::StatusCode};
use sqlx::PgPool;
use super::service::AuthService;
use sha2::{Sha256, Digest};
use sqlx::Row;

pub async fn get_current_user(req: &Request, pool: &PgPool) -> PoemResult<super::service::User> {
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

                    // Check DB
                    let row = sqlx::query(
                        "SELECT user_id, expires_at FROM api_keys WHERE key_hash = $1"
                    )
                    .bind(&key_hash)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

                    if let Some(row) = row {
                        // Check expiration
                        let expires_at: Option<i64> = row.try_get("expires_at").ok();
                        if let Some(exp) = expires_at {
                            if now > exp {
                                return Err(Error::from_string("API Key expired", StatusCode::UNAUTHORIZED));
                            }
                        }

                        // Update last_used_at (async fire-and-forget)
                        let pool_clone = pool.clone();
                        let key_hash_clone = key_hash.clone();
                        tokio::spawn(async move {
                            if let Err(e) = sqlx::query("UPDATE api_keys SET last_used_at = $1 WHERE key_hash = $2")
                                .bind(now)
                                .bind(key_hash_clone)
                                .execute(&pool_clone)
                                .await 
                            {
                                eprintln!("Failed to update api_key stats: {}", e);
                            }
                        });

                        let user_id: String = row.get("user_id");
                        
                        // Fetch full user
                        let auth_service = AuthService::new(pool.clone());
                        let user = auth_service.get_user_by_id(&user_id).await
                            .map_err(|_| Error::from_string("User not found", StatusCode::UNAUTHORIZED))?;
                            
                        return Ok(user);
                    } else {
                        return Err(Error::from_string("Invalid API Key", StatusCode::UNAUTHORIZED));
                    }
                }
            }
        }
    }

    // 2. Try Session Cookie (Fallback)
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
        })
        .ok_or_else(|| Error::from_string("Missing session token", StatusCode::UNAUTHORIZED))?;

    // Validate session and get user
    let auth_service = AuthService::new(pool.clone());
    let user = auth_service.validate_session(&session_token).await
        .map_err(|_| Error::from_string("Invalid or expired session", StatusCode::UNAUTHORIZED))?;

    Ok(user)
}

pub fn get_session_token(req: &Request) -> Option<String> {
    let cookie_header = req.headers().get(poem::http::header::COOKIE)?.to_str().ok()?;
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
        cookie_value.parse().unwrap()
    );
}
