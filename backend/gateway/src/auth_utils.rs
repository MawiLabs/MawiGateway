// Shared utility for extracting user_id from authenticated requests
use poem::{Request, Result as PoemResult, error::Error, http::StatusCode};
use sqlx::PgPool;
use core::auth::AuthService;

pub async fn get_current_user_id(req: &Request, pool: &PgPool) -> PoemResult<String> {
    // Extract session token from cookie
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

    Ok(user.id)
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
    if let Ok(header_value) = cookie_value.parse() {
        response.headers_mut().insert(
            poem::http::header::SET_COOKIE,
            header_value
        );
    } else {
        eprintln!("⚠️ Failed to create session cookie header for token: {}", token);
    }
}
