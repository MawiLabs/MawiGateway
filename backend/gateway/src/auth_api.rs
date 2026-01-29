// Auth API endpoints for registration, login, logout, and user info

use poem::{http::StatusCode, Result as PoemResult, Request, Response};
use poem_openapi::{payload::Json, OpenApi, Object, payload::Response as PoemApiResponse};
use poem::{http::header, error::Error};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use mawi_core::auth::{AuthService, RegisterRequest, LoginRequest};
use crate::api::ApiTags;
use mawi_core::auth::utils::{get_session_token, set_session_cookie};

pub struct AuthApi {
    pub pool: PgPool,
}

#[OpenApi]
impl AuthApi {
    #[oai(path = "/auth/register", method = "post", tag = "ApiTags::Auth")]
    pub async fn register(
        &self,
        req: Json<RegisterReq>,
    ) -> PoemResult<PoemApiResponse<Json<Value>>> {
        let auth_service = AuthService::new(self.pool.clone());
        let register_req = RegisterRequest {
            email: req.0.email.clone(),
            password: req.0.password.clone(),
            name: req.0.name.clone(),
        };

        match auth_service.register(register_req).await {
            Ok(auth_response) => {
                let user_json = json!({
                    "id": auth_response.user.id,
                    "email": auth_response.user.email,
                    "name": auth_response.user.name,
                    "tier": auth_response.user.tier,
                    "monthly_quota_usd": auth_response.user.monthly_quota_usd,
                    "current_usage_usd": auth_response.user.current_usage_usd,
                    "quota_remaining_usd": auth_response.user.monthly_quota_usd - auth_response.user.current_usage_usd,
                    "is_free_tier": auth_response.user.is_free_tier,
                    "organization_id": auth_response.user.org_id,
                });
                
                let user_json_str = serde_json::to_string(&json!({ "user": user_json }))
                    .map_err(|e| poem::Error::from_string(
                        format!("Failed to serialize user response: {}", e), 
                        StatusCode::INTERNAL_SERVER_ERROR
                    ))?;
                    
                let mut resp = Response::builder()
                    .status(StatusCode::CREATED)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(user_json_str);
                
                set_session_cookie(&mut resp, &auth_response.session_token);
                let cookie_val = resp.headers()
                    .get(header::SET_COOKIE)
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or_default()
                    .to_string();

                Ok(PoemApiResponse::new(Json(json!({ "user": user_json })))
                    .status(StatusCode::CREATED)
                    .header(header::SET_COOKIE, cookie_val))
            }
            Err(e) => {
                Err(poem::Error::from_string(e.to_string(), StatusCode::BAD_REQUEST))
            }
        }
    }

    #[oai(path = "/auth/login", method = "post", tag = "ApiTags::Auth")]
    pub async fn login(
        &self,
        req: Json<LoginReq>,
    ) -> PoemResult<PoemApiResponse<Json<Value>>> {
        let auth_service = AuthService::new(self.pool.clone());
        let login_req = LoginRequest {
            email: req.0.email.clone(),
            password: req.0.password.clone(),
        };

        match auth_service.login(login_req).await {
            Ok(auth_response) => {
                let user_json = json!({
                    "id": auth_response.user.id,
                    "email": auth_response.user.email,
                    "name": auth_response.user.name,
                    "tier": auth_response.user.tier,
                    "monthly_quota_usd": auth_response.user.monthly_quota_usd,
                    "current_usage_usd": auth_response.user.current_usage_usd,
                    "quota_remaining_usd": auth_response.user.monthly_quota_usd - auth_response.user.current_usage_usd,
                    "is_free_tier": auth_response.user.is_free_tier,
                    "organization_id": auth_response.user.org_id,
                });
                
                let user_json_str = serde_json::to_string(&json!({ "user": user_json }))
                    .map_err(|e| poem::Error::from_string(
                        format!("Failed to serialize user response: {}", e),
                        StatusCode::INTERNAL_SERVER_ERROR
                    ))?;
                    
                let mut resp = Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(user_json_str);
                
                set_session_cookie(&mut resp, &auth_response.session_token);
                let cookie_val = resp.headers()
                    .get(header::SET_COOKIE)
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or_default()
                    .to_string();
                
                Ok(PoemApiResponse::new(Json(json!({ "user": user_json }))).header(header::SET_COOKIE, cookie_val))
            }
            Err(e) => {
                Err(poem::Error::from_string(e.to_string(), StatusCode::UNAUTHORIZED))
            }
        }
    }

    #[oai(path = "/auth/logout", method = "post", tag = "ApiTags::Auth")]
    pub async fn logout(
        &self,
        req: &Request,
    ) -> PoemResult<PoemApiResponse<Json<String>>> {
        if let Some(token) = get_session_token(req) {
            let auth_service = AuthService::new(self.pool.clone());
            let _ = auth_service.logout(&token).await;
        }
        Ok(PoemApiResponse::new(Json("Logged out successfully".to_string())).header(header::SET_COOKIE, "session_token=; Path=/; HttpOnly; Max-Age=0"))
    }

    #[oai(path = "/auth/me", method = "get", tag = "ApiTags::Auth")]
    pub async fn me(
        &self,
        req: &Request,
    ) -> PoemResult<Json<Value>> {
        let token = match get_session_token(req) {
            Some(t) => t,
            None => {
                return Err(poem::Error::from_string("Not authenticated", StatusCode::UNAUTHORIZED));
            }
        };

        let auth_service = AuthService::new(self.pool.clone());
        match auth_service.validate_session(&token).await {
            Ok(user) => {
                let user_json = json!({
                    "id": user.id,
                    "email": user.email,
                    "name": user.name,
                    "tier": user.tier,
                    "monthly_quota_usd": user.monthly_quota_usd,
                    "current_usage_usd": user.current_usage_usd,
                    "quota_remaining_usd": user.monthly_quota_usd - user.current_usage_usd,
                    "is_free_tier": user.is_free_tier,
                    "organization_id": user.org_id,
                });
                Ok(Json(json!({ "user": user_json })))
            }
            Err(_) => {
                Err(poem::Error::from_string("Session expired", StatusCode::UNAUTHORIZED))
            }
        }
    }
}

#[derive(Debug, Deserialize, Object)]
pub struct RegisterReq {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
}

