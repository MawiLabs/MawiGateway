// User API endpoints for fetching owned providers and models

use poem::{http::StatusCode, Result as PoemResult, Request};
use poem_openapi::{payload::Json, OpenApi, Object};
use crate::api::ApiTags;
use mawi_core::auth::utils::get_session_token;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{PgPool, FromRow, Row};
use mawi_core::auth::{AuthService, UserProfile};
use mawi_core::quota::{QuotaManager, QuotaStatus};
use mawi_core::services::Service;
use rand::Rng;
use sha2::{Sha256, Digest};
use poem_openapi::param::Path;

// DTOs for JSON responses
#[derive(Serialize, FromRow, Object)]
pub struct ProviderInfo {
    id: String,
    name: String,
    provider_type: String,
    has_api_key: bool,  // Indicates if provider has an API key configured
}

#[derive(Serialize, Object)]
pub struct ModelInfo {
    id: String,
    name: String,
    modality: String,  // 'text', 'audio', 'video' - needed for filtering planner models
    tier: String,
    worker_type: String,
    provider: String,  // provider_id so frontend can filter by provider
    health_status: String,  // 'healthy', 'down', 'warning'
    last_error: Option<String>,  //  Error message if unhealthy
}

#[derive(Serialize, Object)]
pub struct ApiKeyInfo {
    id: String,
    name: String,
    prefix: String, // First 8 chars of key for ID
    created_at: String,
    expires_at: Option<String>,
    last_used_at: Option<String>,
}

#[derive(Serialize, Object)]
pub struct CreateApiKeyResponse {
    id: String,
    prefix: String,
    name: String,
    raw_key: String, // ONLY returned on creation
    created_at: String,
    expires_at: Option<String>,
}

#[derive(Serialize, Object)]
pub struct UserProfileResponse {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub tier: String,
    pub monthly_quota_usd: f64,
    pub current_usage_usd: f64,
    pub is_free_tier: bool,
}

#[derive(Serialize, Object)]
pub struct QuotaStatusResponse {
    pub personal_quota: f64,
    pub personal_used: f64,
    pub personal_remaining: f64,
    pub personal_percentage: u8,
    pub org_quota_available: f64,
    pub org_percentage: u8,
    pub total_available: f64,
}

#[derive(poem_openapi::Object, serde::Deserialize)]
pub struct CreateApiKeyRequest {
    name: String,
    expires_in_days: Option<i64>, // None = never
}

pub struct UserApi {
    pub pool: PgPool,
}

#[OpenApi]
impl UserApi {
    // ---------------------------------------------------------------------------
    // Get current user profile
    // ---------------------------------------------------------------------------
    #[oai(path = "/user/me", method = "get", tag = "ApiTags::User")]
    pub async fn me(
        &self,
        req: &Request,
    ) -> PoemResult<Json<UserProfileResponse>> {
        let token = match get_session_token(req) {
            Some(t) => t,
            None => return Err(poem::Error::from_string("Not authenticated", StatusCode::UNAUTHORIZED)),
        };

        let auth_service = AuthService::new(self.pool.clone());
        let user = match auth_service.validate_session(&token).await {
            Ok(u) => u,
            Err(_) => return Err(poem::Error::from_string("Invalid session", StatusCode::UNAUTHORIZED)),
        };

        let profile = AuthService::to_profile(&user);
        
        Ok(Json(UserProfileResponse {
            id: profile.id,
            email: profile.email,
            name: profile.name,
            tier: profile.tier,
            monthly_quota_usd: profile.monthly_quota_usd,
            current_usage_usd: profile.current_usage_usd,
            is_free_tier: profile.is_free_tier,
        }))
    }

    // ---------------------------------------------------------------------------
    // Get current quota status
    // ---------------------------------------------------------------------------
    #[oai(path = "/user/quota", method = "get", tag = "ApiTags::User")]
    pub async fn get_quota(
        &self,
        req: &Request,
    ) -> PoemResult<Json<QuotaStatusResponse>> {
        let token = match get_session_token(req) {
            Some(t) => t,
            None => return Err(poem::Error::from_string("Not authenticated", StatusCode::UNAUTHORIZED)),
        };

        let auth_service = AuthService::new(self.pool.clone());
        let user = match auth_service.validate_session(&token).await {
            Ok(u) => u,
            Err(_) => return Err(poem::Error::from_string("Invalid session", StatusCode::UNAUTHORIZED)),
        };

        let quota_manager = QuotaManager::new(self.pool.clone());
        let status = quota_manager.get_user_quota_status(&user.id).await
            .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

        Ok(Json(QuotaStatusResponse {
            personal_quota: status.personal_quota,
            personal_used: status.personal_used,
            personal_remaining: status.personal_remaining,
            personal_percentage: status.personal_percentage,
            org_quota_available: status.org_quota_available,
            org_percentage: status.org_percentage,
            total_available: status.total_available,
        }))
    }

    // ---------------------------------------------------------------------------
    // Get providers owned by the authenticated user
    // ---------------------------------------------------------------------------
    #[oai(path = "/user/providers", method = "get", tag = "ApiTags::User")]
    pub async fn user_providers(
        &self,
        req: &Request,
    ) -> PoemResult<Json<Vec<ProviderInfo>>> {
        let token = match get_session_token(req) {
            Some(t) => t,
            None => {
                return Err(poem::Error::from_string("Not authenticated", StatusCode::UNAUTHORIZED));
            }
        };

        let auth_service = AuthService::new(self.pool.clone());
        let user = match auth_service.validate_session(&token).await {
            Ok(u) => u,
            Err(_) => {
                return Err(poem::Error::from_string("Invalid session", StatusCode::UNAUTHORIZED));
            }
        };

        // Query providers where user_id = user.id OR user_id IS NULL (shared)
        let rows = sqlx::query_as::<_, ProviderInfo>(
            "SELECT id, name, provider_type, (api_key IS NOT NULL) as has_api_key FROM providers WHERE user_id = $1 OR user_id IS NULL",
        )
        .bind(user.id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

        Ok(Json(rows))
    }

    // ---------------------------------------------------------------------------
    // Get services owned by the authenticated user
    // ---------------------------------------------------------------------------
    #[oai(path = "/user/services", method = "get", tag = "ApiTags::User")]
    pub async fn user_services(
        &self,
        req: &Request,
    ) -> PoemResult<Json<Vec<Service>>> {
        let token = match get_session_token(req) {
            Some(t) => t,
            None => {
                return Err(poem::Error::from_string("Not authenticated", StatusCode::UNAUTHORIZED));
            }
        };

        let auth_service = AuthService::new(self.pool.clone());
        let user = match auth_service.validate_session(&token).await {
            Ok(u) => u,
            Err(_) => {
                return Err(poem::Error::from_string("Invalid session", StatusCode::UNAUTHORIZED));
            }
        };

        let rows = sqlx::query_as::<_, Service>(
            "SELECT * FROM services WHERE user_id = $1 OR user_id IS NULL",
        )
        .bind(user.id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

        Ok(Json(rows))
    }

    // ---------------------------------------------------------------------------
    // Get models owned by the authenticated user
    // ---------------------------------------------------------------------------
    #[oai(path = "/user/models", method = "get", tag = "ApiTags::User")]
    pub async fn user_models(
        &self,
        req: &Request,
    ) -> PoemResult<Json<Vec<ModelInfo>>> {
        let token = match get_session_token(req) {
            Some(t) => t,
            None => {
                return Err(poem::Error::from_string("Not authenticated", StatusCode::UNAUTHORIZED));
            }
        };

        let auth_service = AuthService::new(self.pool.clone());
        let user = match auth_service.validate_session(&token).await {
            Ok(u) => u,
            Err(_) => {
                return Err(poem::Error::from_string("Invalid session", StatusCode::UNAUTHORIZED));
            }
        };

        let rows = sqlx::query(
            "SELECT m.id, m.name, m.modality, m.tier_required as tier, m.worker_type, m.provider_id as provider,
                    CASE 
                        WHEN mh.is_healthy = 1 THEN 'healthy'
                        WHEN mh.is_healthy = 0 THEN 'down'
                        ELSE 'healthy'
                    END as health_status,
                    mh.last_error
             FROM models m
             LEFT JOIN model_health mh ON m.id = mh.model_id
             WHERE m.user_id = $1 OR m.user_id IS NULL",
        )
        .bind(user.id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

        let models: Vec<ModelInfo> = rows.into_iter().map(|row| ModelInfo {
            id: row.try_get("id").unwrap_or_default(),
            name: row.try_get("name").unwrap_or_default(),
            modality: row.try_get("modality").unwrap_or_else(|_| "text".to_string()),
            tier: row.try_get("tier").unwrap_or_default(),
            worker_type: row.try_get("worker_type").unwrap_or_default(),
            provider: row.try_get("provider").unwrap_or_default(),
            health_status: row.try_get("health_status").unwrap_or_else(|_| "healthy".to_string()),
            last_error: row.try_get("last_error").ok(),
        }).collect();

        Ok(Json(models))
    }

    // ---------------------------------------------------------------------------
    // Get user-specific request logs
    // ---------------------------------------------------------------------------
    #[oai(path = "/user/logs", method = "get", tag = "ApiTags::User")]
    pub async fn user_logs(
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
        let user = match auth_service.validate_session(&token).await {
            Ok(u) => u,
            Err(_) => {
                return Err(poem::Error::from_string("Invalid session", StatusCode::UNAUTHORIZED));
            }
        };

        // Get request logs for services owned by this user
        let logs = sqlx::query(
            "SELECT rl.id, rl.service_name, rl.model_id, rl.provider_type, rl.latency_ms, rl.latency_us, rl.status, 
                    CAST(rl.created_at AS TEXT) as created_at_str,
                    rl.tokens_prompt, rl.tokens_completion, rl.tokens_total, rl.cost_usd, rl.error_message, rl.failover_count
             FROM request_logs rl
             INNER JOIN services s ON s.name = rl.service_name  
             WHERE s.user_id = $1
             ORDER BY rl.created_at DESC
             LIMIT 100"
        )
        .bind(&user.id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

        let result: Vec<serde_json::Value> = logs.into_iter().map(|row: sqlx::postgres::PgRow| {
            // Parse created_at manually to ensure correct date
            let created_at_str: Option<String> = row.try_get("created_at_str").ok();
            let created_at = created_at_str
                .and_then(|s| s.parse::<i64>().ok())
                .map(|ts| chrono::NaiveDateTime::from_timestamp_opt(ts, 0).unwrap_or_default().to_string());

            serde_json::json!({
                "id": row.try_get::<String, _>("id").ok(),
                "service_name": row.try_get::<String, _>("service_name").ok(),
                "model_id": row.try_get::<String, _>("model_id").ok(),
                "provider_type": row.try_get::<String, _>("provider_type").ok(),
                "latency_ms": row.try_get::<i64, _>("latency_ms").ok(),
                "latency_us": row.try_get::<i64, _>("latency_us").ok(),
                "status": row.try_get::<String, _>("status").ok(),
                "created_at": created_at,
                "tokens_prompt": row.try_get::<i64, _>("tokens_prompt").ok(),
                "tokens_completion": row.try_get::<i64, _>("tokens_completion").ok(),
                "tokens_total": row.try_get::<i64, _>("tokens_total").ok(),
                "cost_usd": row.try_get::<f64, _>("cost_usd").ok(),
                "error_message": row.try_get::<String, _>("error_message").ok(),
                "failover_count": row.try_get::<i64, _>("failover_count").ok(),
            })
        }).collect();

        Ok(Json(json!(result)))
    }

    // ---------------------------------------------------------------------------
    // Get user-specific analytics
    // ---------------------------------------------------------------------------
    #[oai(path = "/user/analytics", method = "get", tag = "ApiTags::User")]
    pub async fn user_analytics(
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
        let user = match auth_service.validate_session(&token).await {
            Ok(u) => u,
            Err(_) => {
                return Err(poem::Error::from_string("Invalid session", StatusCode::UNAUTHORIZED));
            }
        };

        // Get analytics for services owned by this user
        let stats_row = sqlx::query(
            "SELECT 
                COUNT(*) as total_requests,
                AVG(CASE WHEN status = 'success' THEN 1.0 ELSE 0.0 END) as success_rate,
                AVG(latency_ms) as avg_latency,
                AVG(CASE WHEN status = 'error' THEN 1.0 ELSE 0.0 END) as error_rate
             FROM request_logs rl
             INNER JOIN services s ON s.name = rl.service_name
             WHERE s.user_id = $1"
        )
        .bind(&user.id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

        // Get top models
        let top_models = sqlx::query_as::<_, (String, i64)>(
            "SELECT rl.model_id as model_name, COUNT(*) as count
             FROM request_logs rl
             INNER JOIN services s ON s.name = rl.service_name
             WHERE s.user_id = $1
             GROUP BY rl.model_id
             ORDER BY count DESC
             LIMIT 5"
        )
        .bind(&user.id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

        let analytics = json!({
            "total_requests": stats_row.try_get::<i64, _>("total_requests").unwrap_or(0),
            "success_rate": stats_row.try_get::<f64, _>("success_rate").unwrap_or(0.0),
            "avg_latency": stats_row.try_get::<f64, _>("avg_latency").unwrap_or(0.0),
            "error_rate": stats_row.try_get::<f64, _>("error_rate").unwrap_or(0.0),
            "top_models": top_models.into_iter().map(|row: (String, i64)| {
                json!({
                    "model_name": row.0,
                    "count": row.1
                })
            }).collect::<Vec<_>>()
        });

        Ok(Json(analytics))
    }

    // ---------------------------------------------------------------------------
    // Manual health check for a specific model
    // ---------------------------------------------------------------------------
    #[oai(path = "/user/models/:model_id/health", method = "post", tag = "ApiTags::User")]
    pub async fn refresh_model_health(
        &self,
        req: &Request,
        model_id: poem_openapi::param::Path<String>,
    ) -> PoemResult<Json<Value>> {
        let token = match get_session_token(req) {
            Some(t) => t,
            None => {
                return Err(poem::Error::from_string("Not authenticated", StatusCode::UNAUTHORIZED));
            }
        };

        let auth_service = AuthService::new(self.pool.clone());
        let _user = match auth_service.validate_session(&token).await {
            Ok(u) => u,
            Err(_) => {
                return Err(poem::Error::from_string("Invalid session", StatusCode::UNAUTHORIZED));
            }
        };

        // Get model details
        let model = sqlx::query_as::<_, (String, String, String)>(
            "SELECT name, provider_id, modality FROM models WHERE id = $1"
        )
        .bind(&model_id.0)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| poem::Error::from_string(format!("Model not found: {}", e), StatusCode::NOT_FOUND))?;

        let (model_name, provider_id, modality) = model;

        // Use HealthMonitor to check health
        let monitor = crate::health::HealthMonitor::new(self.pool.clone());
        let health = monitor.ping_single_model(&model_id.0, &model_name, &provider_id, &modality).await;

        // Update health table
        sqlx::query(
            "INSERT INTO model_health 
             (model_id, is_healthy, last_check, response_time_ms, consecutive_failures, last_error)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (model_id) DO UPDATE SET 
               is_healthy = EXCLUDED.is_healthy,
               last_check = EXCLUDED.last_check,
               response_time_ms = EXCLUDED.response_time_ms,
               consecutive_failures = EXCLUDED.consecutive_failures,
               last_error = EXCLUDED.last_error"
        )
        .bind(&model_id.0)
        .bind(health.is_healthy as i64)
        .bind(chrono::Utc::now().timestamp())
        .bind(health.response_time_ms)
        .bind(health.consecutive_failures)
        .bind(&health.last_error)
        .execute(&self.pool)
        .await
        .map_err(|e: sqlx::Error| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

        Ok(Json(json!({
            "model_id": model_id.0,
            "is_healthy": health.is_healthy,
            "response_time_ms": health.response_time_ms,
            "last_error": health.last_error
        })))
    }

    // ---------------------------------------------------------------------------
    // API Key Management
    // ---------------------------------------------------------------------------

    // List API Keys
    #[oai(path = "/user/api-keys", method = "get", tag = "ApiTags::User")]
    pub async fn list_api_keys(
        &self,
        req: &Request,
    ) -> PoemResult<Json<Vec<ApiKeyInfo>>> {
        let token = match get_session_token(req) {
            Some(t) => t,
            None => return Err(poem::Error::from_string("Not authenticated", StatusCode::UNAUTHORIZED)),
        };

        let auth_service = AuthService::new(self.pool.clone());
        let user = match auth_service.validate_session(&token).await {
            Ok(u) => u,
            Err(_) => return Err(poem::Error::from_string("Invalid session", StatusCode::UNAUTHORIZED)),
        };

        let rows = sqlx::query(
            "SELECT id, name, CAST(created_at AS TEXT) as created_at_str, CAST(expires_at AS TEXT) as expires_at_str, CAST(last_used_at AS TEXT) as last_used_at_str 
             FROM api_keys 
             WHERE user_id = $1 
             ORDER BY created_at DESC"
        )
        .bind(&user.id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

        let keys = rows.into_iter().map(|row| {
            let id: String = row.get("id");
            let prefix = if id.len() > 10 {
                id.chars().take(12).collect::<String>() + "..."
            } else {
                id.clone()
            };

            let parse_ts = |col: &str| -> Option<String> {
                let s: Option<String> = row.try_get(col).ok();
                s.and_then(|s| s.parse::<i64>().ok())
                 .map(|ts| chrono::NaiveDateTime::from_timestamp_opt(ts, 0).unwrap_or_default().to_string())
            };
            
            // For logs
            let ca_str: Option<String> = row.try_get("created_at_str").ok();
            println!("Key {} raw created_at_str: {:?}", id, ca_str);

            ApiKeyInfo {
                id,
                name: row.get("name"),
                prefix, 
                created_at: parse_ts("created_at_str").unwrap_or_default(),
                expires_at: parse_ts("expires_at_str"),
                last_used_at: parse_ts("last_used_at_str"),
            }
        }).collect();

        Ok(Json(keys))
    }

    // Create API Key
    #[oai(path = "/user/api-keys", method = "post", tag = "ApiTags::User")]
    pub async fn create_api_key(
        &self,
        req: &Request,
        body: Json<CreateApiKeyRequest>,
    ) -> PoemResult<Json<CreateApiKeyResponse>> {
        let token = match get_session_token(req) {
            Some(t) => t,
            None => return Err(poem::Error::from_string("Not authenticated", StatusCode::UNAUTHORIZED)),
        };

        let auth_service = AuthService::new(self.pool.clone());
        let user = match auth_service.validate_session(&token).await {
            Ok(u) => u,
            Err(_) => return Err(poem::Error::from_string("Invalid session", StatusCode::UNAUTHORIZED)),
        };

        // Generate ID and Key
        let raw_key = format!("sk_live_{}", uuid::Uuid::new_v4().simple());
        // For actual security, we should generate a high-entropy random string
        // But UUID is decent for this MVP. Let's make it stronger.
        let random_bytes: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let key_secret = format!("sk_{}", random_bytes);
        
        // Use the first 10 characters of randomness for the ID to ensure it matches visual prefix
        // e.g. ID: sk_ABC1234567
        // Collision chance (62^10) is negligible.
        let prefix_random: String = random_bytes.chars().take(10).collect();
        let db_id = format!("sk_{}", prefix_random);
        
        // Hash the secret
        let mut hasher = Sha256::new();
        hasher.update(key_secret.as_bytes());
        let key_hash = hex::encode(hasher.finalize());

        let created_at = chrono::Utc::now().timestamp();
        let expires_at = body.expires_in_days.map(|days| 
            created_at + (days * 24 * 60 * 60)
        );

        sqlx::query(
            "INSERT INTO api_keys (id, user_id, name, key_hash, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(&db_id)
        .bind(&user.id)
        .bind(&body.name)
        .bind(&key_hash)
        .bind(created_at)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

        Ok(Json(CreateApiKeyResponse {
            id: db_id,
            prefix: key_secret.chars().take(12).collect::<String>() + "...",
            name: body.name.clone(),
            raw_key: key_secret,
            created_at: chrono::NaiveDateTime::from_timestamp_opt(created_at, 0).unwrap_or_default().to_string(),
            expires_at: expires_at.map(|ts| chrono::NaiveDateTime::from_timestamp_opt(ts, 0).unwrap_or_default().to_string()),
        }))
    }

    // Revoke API Key
    #[oai(path = "/user/api-keys/:id", method = "delete", tag = "ApiTags::User")]
    pub async fn delete_api_key(
        &self,
        req: &Request,
        id: Path<String>,
    ) -> PoemResult<Json<bool>> {
        println!("DELETE handler hit for ID: {}", id.0);
        let token = match get_session_token(req) {
            Some(t) => t,
            None => return Err(poem::Error::from_string("Not authenticated", StatusCode::UNAUTHORIZED)),
        };

        let auth_service = AuthService::new(self.pool.clone());
        let user = match auth_service.validate_session(&token).await {
            Ok(u) => u,
            Err(_) => return Err(poem::Error::from_string("Invalid session", StatusCode::UNAUTHORIZED)),
        };

        println!("Deleting API key: {} for user: {}", id.0, user.id);

        // Check if key exists first
        let exists = sqlx::query("SELECT id FROM api_keys WHERE id = $1")
            .bind(&id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

        if exists.is_none() {
            println!("Key {} IS NOT IN DATABASE at all", id.0);
        } else {
            println!("Key {} exists in DB. Checking ownership...", id.0);
        }

        let result = sqlx::query(
            "DELETE FROM api_keys WHERE id = $1 AND user_id = $2"
        )
        .bind(&id.0)
        .bind(&user.id)
        .execute(&self.pool)
        .await
        .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

        if result.rows_affected() == 0 {
            println!("Delete failed: rows_affected=0. User mismatch likely.");
            return Err(poem::Error::from_string("Key not found or access denied", StatusCode::NOT_FOUND));
        }

        Ok(Json(true))
    }
}
