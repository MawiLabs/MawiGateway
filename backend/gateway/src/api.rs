use mawi_core::models::{
    CreateModel, CreateProvider, Model, Provider, UpdateModel, UpdateProvider,
};
use mawi_core::services::{
    AssignModel, CreateService, Service, UpdateModelAssignment, UpdateService,
};
use poem_openapi::{param::Path, param::Query, payload::Json, OpenApi, Tags};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::OnceLock;
use uuid::Uuid;

// Cache for environment variable presence to avoid syscalls in hot loops
static ENV_KEYS_PRESENT: OnceLock<HashMap<String, bool>> = OnceLock::new();

fn check_env_key(key: &str) -> bool {
    let map = ENV_KEYS_PRESENT.get_or_init(|| {
        let keys = vec![
            "MG_OPENAI_API_KEY",
            "MG_ANTHROPIC_API_KEY",
            "MG_PERPLEXITY_API_KEY",
            "MG_MISTRAL_API_KEY",
            "MG_GEMINI_API_KEY",
            "MG_GOOGLE_API_KEY",
            "MG_AZURE_OPENAI_API_KEY",
            "MG_ELEVENLABS_API_KEY",
            "MG_XAI_API_KEY",
            "MG_DEEPSEEK_API_KEY",
            "MG_HUME_API_KEY",
            "MG_RUNWAY_API_KEY",
            "MG_KLING_API_KEY",
            "MG_LUMA_API_KEY",
            "MG_PIKA_API_KEY",
            "MG_MINIMAX_API_KEY",
            "MG_BYTEDANCE_API_KEY",
        ];
        let mut m = HashMap::new();
        for k in keys {
            m.insert(k.to_string(), std::env::var(k).is_ok());
        }
        m
    });
    *map.get(key).unwrap_or(&false)
}

#[derive(Tags)]
pub enum ApiTags {
    Providers,
    Models,
    Services,
    System,
    Analytics,
    Auth,
    User,
    Organizations,
}

// Response struct that includes has_api_key indicator
#[derive(Debug, Serialize, poem_openapi::Object)]
pub struct ProviderResponse {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub api_endpoint: Option<String>,
    pub api_version: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<i64>,
    pub has_api_key: bool,
    pub icon_url: Option<String>,
}

impl From<Provider> for ProviderResponse {
    fn from(p: Provider) -> Self {
        let has_db_key = p.api_key.as_deref().map(|k| !k.is_empty()).unwrap_or(false);

        let has_env_key = if !has_db_key {
            match p.provider_type.to_lowercase().as_str() {
                "openai" => check_env_key("MG_OPENAI_API_KEY"),
                "anthropic" => check_env_key("MG_ANTHROPIC_API_KEY"),
                "perplexity" => check_env_key("MG_PERPLEXITY_API_KEY"),
                "mistral" => check_env_key("MG_MISTRAL_API_KEY"),
                "google" | "gemini" => {
                    check_env_key("MG_GEMINI_API_KEY") || check_env_key("MG_GOOGLE_API_KEY")
                }
                "azure" => check_env_key("MG_AZURE_OPENAI_API_KEY"),
                "elevenlabs" => check_env_key("MG_ELEVENLABS_API_KEY"),
                "xai" => check_env_key("MG_XAI_API_KEY"),
                "deepseek" => check_env_key("MG_DEEPSEEK_API_KEY"),
                "hume" | "humeai" | "hume-ai" => check_env_key("MG_HUME_API_KEY"),
                "runway" => check_env_key("MG_RUNWAY_API_KEY"),
                "kling" | "kuaishou" => check_env_key("MG_KLING_API_KEY"),
                "luma" | "lumaai" | "luma-ai" => check_env_key("MG_LUMA_API_KEY"),
                "pika" | "pikalabs" => check_env_key("MG_PIKA_API_KEY"),
                "minimax" | "hailuo" => check_env_key("MG_MINIMAX_API_KEY"),
                "bytedance" | "seedance" => check_env_key("MG_BYTEDANCE_API_KEY"),
                _ => false,
            }
        } else {
            false
        };

        Self {
            id: p.id,
            name: p.name,
            provider_type: p.provider_type,
            api_endpoint: p.api_endpoint,
            api_version: p.api_version,
            description: p.description,
            created_at: p.created_at,
            has_api_key: has_db_key || has_env_key,
            icon_url: p.icon_url,
        }
    }
}

pub struct ModelsApi {
    pub pool: PgPool,
}

#[OpenApi]
impl ModelsApi {
    // ==================== PROVIDERS ====================

    /// List all providers (paginated, `?limit=&offset=`, max 200; #40)
    #[oai(path = "/providers", method = "get", tag = "ApiTags::Providers")]
    async fn list_providers(
        &self,
        Query(limit): Query<Option<i64>>,
        Query(offset): Query<Option<i64>>,
    ) -> poem::Result<Json<Vec<ProviderResponse>>> {
        let page = crate::pagination::Pagination::from_parts(limit, offset);
        let providers_result: Vec<Provider> =
            sqlx::query_as("SELECT * FROM providers ORDER BY name LIMIT $1 OFFSET $2")
                .bind(page.limit)
                .bind(page.offset)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    poem::error::Error::from_string(
                        format!("Database error: {}", e),
                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?;
        let providers = providers_result;
        Ok(Json(
            providers.into_iter().map(ProviderResponse::from).collect(),
        ))
    }

    /// Get model group by ID
    #[oai(path = "/providers/:id", method = "get", tag = "ApiTags::Providers")]
    async fn get_provider(&self, id: Path<String>) -> poem::Result<Json<ProviderResponse>> {
        let provider: Option<Provider> = sqlx::query_as("SELECT * FROM providers WHERE id = $1")
            .bind(&id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Database error: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        provider
            .ok_or_else(|| {
                poem::error::Error::from_string(
                    format!("Provider '{}' not found", id.0),
                    poem::http::StatusCode::NOT_FOUND,
                )
            })
            .map(|p| Json(ProviderResponse::from(p)))
    }

    /// Create model group
    #[oai(path = "/providers", method = "post", tag = "ApiTags::Providers")]
    async fn create_provider(
        &self,
        req: Json<CreateProvider>,
        poem_req: &poem::Request,
    ) -> poem::Result<Json<Provider>> {
        let id = Uuid::new_v4().to_string();

        // Validate inputs
        if req.name.trim().is_empty() || req.name.len() > 100 {
            return Err(poem::error::Error::from_string(
                "Provider name must be between 1 and 100 characters",
                poem::http::StatusCode::BAD_REQUEST,
            ));
        }

        eprintln!(
            "Creating provider: name={}, type={}",
            req.name, req.provider_type
        );

        // Extract user_id from session (injected by AuthMiddleware)
        let user = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .ok_or_else(|| {
                poem::error::Error::from_string(
                    "Authentication required",
                    poem::http::StatusCode::UNAUTHORIZED,
                )
            })?;
        let user_id = &user.id;

        // Encrypt API key if present
        let encrypted_key = if let Some(key) = &req.api_key {
            Some(mawi_core::security::encrypt_key(key).map_err(|e| {
                poem::error::Error::from_string(
                    format!("Encryption failed: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?)
        } else {
            None
        };

        sqlx::query("INSERT INTO providers (id, name, provider_type, api_endpoint, api_version, api_key, description, icon_url, user_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
            .bind(&id)
            .bind(&req.name)
            .bind(&req.provider_type)
            .bind(&req.api_endpoint)
            .bind(&req.api_version)
            .bind(&encrypted_key)
            .bind(&req.description)
            .bind(&req.icon_url)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database insert error: {}", e);
                poem::error::Error::from_string(
                    format!("Failed to create provider: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR
                )
            })?;

        let provider: Provider = sqlx::query_as("SELECT * FROM providers WHERE id = $1")
            .bind(&id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database fetch error: {}", e);
                poem::error::Error::from_string(
                    format!("Failed to fetch provider: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        Ok(Json(provider))
    }

    /// Update model group
    #[oai(path = "/providers/:id", method = "put", tag = "ApiTags::Providers")]
    async fn update_provider(
        &self,
        id: Path<String>,
        req: Json<UpdateProvider>,
    ) -> poem::Result<Json<Provider>> {
        // Check exists
        let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM providers WHERE id = $1")
            .bind(&id.0)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0)
            > 0;

        if !exists {
            return Err(poem::error::Error::from_string(
                format!("Provider '{}' not found", id.0),
                poem::http::StatusCode::NOT_FOUND,
            ));
        }

        // Build dynamic update query
        let mut updates = Vec::new();
        let mut params: Vec<String> = Vec::new();
        let mut param_idx = 1;

        if let Some(name) = &req.name {
            updates.push(format!("name = ${}", param_idx));
            param_idx += 1;
            params.push(name.clone());
        }
        if let Some(provider_type) = &req.provider_type {
            updates.push(format!("provider_type = ${}", param_idx));
            param_idx += 1;
            params.push(provider_type.clone());
        }
        if let Some(api_endpoint) = &req.api_endpoint {
            updates.push(format!("api_endpoint = ${}", param_idx));
            param_idx += 1;
            params.push(api_endpoint.clone());
        }
        if let Some(api_version) = &req.api_version {
            updates.push(format!("api_version = ${}", param_idx));
            param_idx += 1;
            params.push(api_version.clone());
        }
        if let Some(api_key) = &req.api_key {
            updates.push(format!("api_key = ${}", param_idx));
            param_idx += 1;

            // Encrypt key!
            let encrypted = mawi_core::security::encrypt_key(api_key).map_err(|e| {
                poem::error::Error::from_string(
                    format!("Failed to encrypt key: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;
            params.push(encrypted);
        }
        if let Some(description) = &req.description {
            updates.push(format!("description = ${}", param_idx));
            param_idx += 1;
            params.push(description.clone());
        }
        if let Some(icon_url) = &req.icon_url {
            updates.push(format!("icon_url = ${}", param_idx));
            param_idx += 1;
            params.push(icon_url.clone());
        }

        if !updates.is_empty() {
            let query = format!(
                "UPDATE providers SET {} WHERE id = ${}",
                updates.join(", "),
                param_idx
            );
            let mut q = sqlx::query(&query);
            for param in params {
                q = q.bind(param);
            }
            q = q.bind(&id.0);
            q.execute(&self.pool).await.map_err(|e| {
                poem::error::Error::from_string(
                    format!("Failed to update provider: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;
        }

        let group: Provider = sqlx::query_as("SELECT * FROM providers WHERE id = $1")
            .bind(&id.0)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Failed to fetch updated provider: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        Ok(Json(group))
    }

    /// Delete provider
    #[oai(path = "/providers/:id", method = "delete", tag = "ApiTags::Providers")]
    async fn delete_provider(
        &self,
        id: Path<String>,
        poem_req: &poem::Request,
    ) -> poem::Result<Json<String>> {
        // Extract user_id from session (injected by AuthMiddleware)
        let user = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .ok_or_else(|| {
                poem::error::Error::from_string(
                    "Authentication required",
                    poem::http::StatusCode::UNAUTHORIZED,
                )
            })?;
        let user_id = &user.id;

        // Delete only if owned by this user
        let result = sqlx::query("DELETE FROM providers WHERE id = $1 AND user_id = $2")
            .bind(&id.0)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Failed to delete: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        if result.rows_affected() == 0 {
            return Err(poem::error::Error::from_string(
                format!(
                    "Provider '{}' not found or you don't have permission to delete it",
                    id.0
                ),
                poem::http::StatusCode::NOT_FOUND,
            ));
        }

        Ok(Json("Provider deleted".to_string()))
    }

    // ==================== MODELS ====================

    /// List all models with health status (paginated, `?limit=&offset=`, max 200; #40)
    #[oai(path = "/models", method = "get", tag = "ApiTags::Models")]
    async fn list_models(
        &self,
        Query(limit): Query<Option<i64>>,
        Query(offset): Query<Option<i64>>,
    ) -> poem::Result<Json<Vec<serde_json::Value>>> {
        #[derive(sqlx::FromRow)]
        struct ModelWithHealth {
            id: String,
            name: String,
            provider_id: String, // mapped from "provider" in struct but "provider_id" in DB... wait Model struct has #[sqlx(rename="provider_id")] pub provider: String
            modality: String,
            description: Option<String>,
            api_endpoint: Option<String>,
            api_version: Option<String>,
            api_key: Option<String>,
            created_at: Option<i64>,
            is_healthy: Option<bool>,
            last_error: Option<String>,
        }

        let page = crate::pagination::Pagination::from_parts(limit, offset);
        let models: Vec<ModelWithHealth> = sqlx::query_as(
            "SELECT m.id, m.name, m.provider_id, m.modality, m.description,
             m.api_endpoint, m.api_version, m.api_key, m.created_at,
             h.is_healthy, h.last_error
             FROM models m
             LEFT JOIN model_health h ON m.id = h.model_id
             ORDER BY m.name
             LIMIT $1 OFFSET $2",
        )
        .bind(page.limit)
        .bind(page.offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            poem::error::Error::from_string(
                format!("Database error: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        let result: Vec<serde_json::Value> = models
            .iter()
            .map(|m| {
                let health_status = if m.is_healthy.unwrap_or(true) {
                    "healthy"
                } else {
                    let err = m.last_error.as_deref().unwrap_or("");
                    if err.contains("Rate Limited")
                        || err.contains("429")
                        || err.contains("Client Error")
                    {
                        "warning"
                    } else {
                        "unhealthy"
                    }
                };

                // Mask API key for security - only show last 4 characters
                let masked_api_key = m
                    .api_key
                    .as_ref()
                    .map(|k| mawi_core::utils::mask_api_key(k));

                serde_json::json!({
                    "id": m.id,
                    "name": m.name,
                    "provider": m.provider_id,
                    "modality": m.modality,
                    "description": m.description,
                    "api_endpoint": m.api_endpoint,
                    "api_version": m.api_version,
                    "api_key_masked": masked_api_key,
                    "has_api_key": m.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false),
                    "created_at": m.created_at,
                    "is_healthy": m.is_healthy.unwrap_or(true),
                    "health_status": health_status,
                    "last_error": m.last_error
                })
            })
            .collect();

        Ok(Json(result))
    }

    /// Get model by ID
    #[oai(path = "/models/:id", method = "get", tag = "ApiTags::Models")]
    async fn get_model(&self, id: Path<String>) -> poem::Result<Json<Model>> {
        let model: Option<Model> = sqlx::query_as("SELECT * FROM models WHERE id = $1")
            .bind(&id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Database error: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        model
            .ok_or_else(|| {
                poem::error::Error::from_string(
                    format!("Model '{}' not found", id.0),
                    poem::http::StatusCode::NOT_FOUND,
                )
            })
            .map(Json)
    }

    /// Create model
    #[oai(path = "/models", method = "post", tag = "ApiTags::Models")]
    async fn create_model(
        &self,
        req: Json<CreateModel>,
        poem_req: &poem::Request,
    ) -> poem::Result<Json<Model>> {
        // Validate inputs
        if req.name.trim().is_empty() || req.name.len() > 100 {
            return Err(poem::error::Error::from_string(
                "Model name must be between 1 and 100 characters",
                poem::http::StatusCode::BAD_REQUEST,
            ));
        }

        eprintln!(
            "Creating model: name={}, provider_id={}",
            req.name, req.provider
        );

        // Extract user_id from session (injected by AuthMiddleware)
        let user = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .ok_or_else(|| {
                poem::error::Error::from_string(
                    "Authentication required",
                    poem::http::StatusCode::UNAUTHORIZED,
                )
            })?;
        let user_id = user.id.clone();

        // Check for duplicate model name within the same provider
        let existing: Option<Model> =
            sqlx::query_as("SELECT * FROM models WHERE name = $1 AND provider_id = $2")
                .bind(&req.name)
                .bind(&req.provider)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    eprintln!("Database check error: {}", e);
                    poem::error::Error::from_string(
                        format!("Database error: {}", e),
                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?;

        if existing.is_some() {
            eprintln!(
                "Duplicate model rejected: {} already exists for this provider",
                req.name
            );
            return Err(poem::error::Error::from_string(
                format!("Model '{}' already exists for this provider", req.name),
                poem::http::StatusCode::CONFLICT,
            ));
        }

        let id = Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().timestamp();

        sqlx::query("INSERT INTO models (id, name, provider_id, modality, description, api_endpoint, api_version, api_key, created_at, tier_required, worker_type, user_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'A', 'text', $10)")
            .bind(&id)
            .bind(&req.name)
            .bind(&req.provider)
            .bind(&req.modality)
            .bind(&req.description)
            .bind(&req.api_endpoint)
            .bind(&req.api_version)
            .bind(&req.api_key)
            .bind(created_at)
            .bind(&user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database error: {}", e);
                poem::error::Error::from_string(
                    format!("Failed to create model: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR
                )
            })?;

        let model = Model {
            id,
            name: req.name.clone(),
            provider: req.provider.clone(),
            modality: req.modality.clone(),
            description: req.description.clone(),

            // Pricing metadata
            cost_per_1k_tokens: req.cost_per_1k_tokens,
            cost_per_1k_input_tokens: req.cost_per_1k_input_tokens,
            cost_per_1k_output_tokens: req.cost_per_1k_output_tokens,
            tier: req.tier.clone().unwrap_or_else(|| "standard".to_string()),

            // Performance metrics (will be updated from actual requests)
            avg_latency_ms: 0,
            avg_ttft_ms: 0,
            max_tps: 0,

            context_window: 8192, // Default context window

            api_endpoint: req.api_endpoint.clone(),
            api_version: req.api_version.clone(),
            api_key: req.api_key.clone(),
            created_at: Some(created_at),
            tier_required: "A".to_string(),
            worker_type: req.modality.clone(),
            created_by: None,
            user_id: Some(user_id),
        };

        Ok(Json(model))
    }

    /// Update model
    #[oai(path = "/models/:id", method = "put", tag = "ApiTags::Models")]
    async fn update_model(
        &self,
        id: Path<String>,
        req: Json<UpdateModel>,
    ) -> poem::Result<Json<Model>> {
        let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM models WHERE id = $1")
            .bind(&id.0)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0)
            > 0;

        if !exists {
            return Err(poem::error::Error::from_string(
                format!("Model '{}' not found", id.0),
                poem::http::StatusCode::NOT_FOUND,
            ));
        }

        let mut updates = Vec::new();
        let mut params: Vec<String> = Vec::new();
        let mut param_idx = 1;

        if let Some(name) = &req.name {
            updates.push(format!("name = ${}", param_idx));
            param_idx += 1;
            params.push(name.clone());
        }
        if let Some(provider_id) = &req.provider {
            updates.push(format!("provider_id = ${}", param_idx));
            param_idx += 1;
            params.push(provider_id.clone());
        }
        if let Some(modality) = &req.modality {
            updates.push(format!("modality = ${}", param_idx));
            param_idx += 1;
            params.push(modality.clone());
        }
        if let Some(description) = &req.description {
            updates.push(format!("description = ${}", param_idx));
            param_idx += 1;
            params.push(description.clone());
        }
        if let Some(api_endpoint) = &req.api_endpoint {
            updates.push(format!("api_endpoint = ${}", param_idx));
            param_idx += 1;
            params.push(api_endpoint.clone());
        }
        if let Some(api_version) = &req.api_version {
            updates.push(format!("api_version = ${}", param_idx));
            param_idx += 1;
            params.push(api_version.clone());
        }
        if let Some(api_key) = &req.api_key {
            updates.push(format!("api_key = ${}", param_idx));
            param_idx += 1;
            params.push(api_key.clone());
        }

        if !updates.is_empty() {
            let query = format!(
                "UPDATE models SET {} WHERE id = ${}",
                updates.join(", "),
                param_idx
            );
            let mut q = sqlx::query(&query);
            for param in params {
                q = q.bind(param);
            }
            q = q.bind(&id.0);
            q.execute(&self.pool).await.map_err(|e| {
                poem::error::Error::from_string(
                    format!("Failed to update model: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;
        }

        let model: Model = sqlx::query_as("SELECT * FROM models WHERE id = $1")
            .bind(&id.0)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Failed to fetch updated model: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        Ok(Json(model))
    }

    /// Delete model
    #[oai(path = "/models/:id", method = "delete", tag = "ApiTags::Models")]
    async fn delete_model(
        &self,
        id: Path<String>,
        poem_req: &poem::Request,
    ) -> poem::Result<Json<String>> {
        // Extract user_id from session (injected by AuthMiddleware)
        let user = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .ok_or_else(|| {
                poem::error::Error::from_string(
                    "Authentication required",
                    poem::http::StatusCode::UNAUTHORIZED,
                )
            })?;
        let user_id = &user.id;

        // Delete only if owned by this user
        let result = sqlx::query("DELETE FROM models WHERE id = $1 AND user_id = $2")
            .bind(&id.0)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Failed to delete: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        if result.rows_affected() == 0 {
            return Err(poem::error::Error::from_string(
                format!(
                    "Model '{}' not found or you don't have permission to delete it",
                    id.0
                ),
                poem::http::StatusCode::NOT_FOUND,
            ));
        }

        Ok(Json("Model deleted".to_string()))
    }

    // Helper: Fetch full service object
    async fn fetch_full_service(&self, name: &str) -> poem::Result<Service> {
        let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Database error: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        service.ok_or_else(|| {
            poem::error::Error::from_string(
                format!("Service '{}' not found", name),
                poem::http::StatusCode::NOT_FOUND,
            )
        })
    }
    /// List all services (paginated, `?limit=&offset=`, max 200; #40)
    #[oai(path = "/services", method = "get", tag = "ApiTags::Services")]
    async fn list_services(
        &self,
        Query(limit): Query<Option<i64>>,
        Query(offset): Query<Option<i64>>,
    ) -> poem::Result<Json<Vec<Service>>> {
        let page = crate::pagination::Pagination::from_parts(limit, offset);
        let services: Vec<Service> = sqlx::query_as(
            "SELECT * FROM services ORDER BY name LIMIT $1 OFFSET $2",
        )
        .bind(page.limit)
        .bind(page.offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            poem::error::Error::from_string(
                format!("Database error: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        Ok(Json(services))
    }

    /// Get service by name
    #[oai(path = "/services/:name", method = "get", tag = "ApiTags::Services")]
    async fn get_service(&self, name: Path<String>) -> poem::Result<Json<Service>> {
        let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE name = $1")
            .bind(&name.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Database error: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        service
            .ok_or_else(|| {
                poem::error::Error::from_string(
                    format!("Service '{}' not found", name.0),
                    poem::http::StatusCode::NOT_FOUND,
                )
            })
            .map(Json)
    }

    /// Create service
    #[oai(path = "/services", method = "post", tag = "ApiTags::Services")]
    async fn create_service(
        &self,
        req: Json<CreateService>,
        poem_req: &poem::Request,
    ) -> poem::Result<Json<Service>> {
        let strategy = req.strategy.clone().unwrap_or("weighted".to_string());
        let guardrails_json = serde_json::to_string(&req.guardrails).unwrap_or("[]".to_string());

        // Validate inputs
        if req.name.trim().is_empty() || req.name.len() > 100 {
            return Err(poem::error::Error::from_string(
                "Service name must be between 1 and 100 characters",
                poem::http::StatusCode::BAD_REQUEST,
            ));
        }
        let valid_types = ["agentic", "pool"];
        if !valid_types.contains(&req.service_type.to_lowercase().as_str()) {
            return Err(poem::error::Error::from_string(
                format!("Invalid service type. Must be one of: {:?}", valid_types),
                poem::http::StatusCode::BAD_REQUEST,
            ));
        }

        eprintln!(
            "Creating service: name={}, type={}",
            req.name, req.service_type
        );

        // Extract user_id from session (injected by AuthMiddleware)
        let user = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .ok_or_else(|| {
                poem::error::Error::from_string(
                    "Authentication required",
                    poem::http::StatusCode::UNAUTHORIZED,
                )
            })?;
        let user_id = &user.id;

        // Insert service with agentic fields + aliases + cache settings.
        // Migration 033's trigger rejects the INSERT (with a
        // unique_violation) if any alias collides with another
        // service's name or alias. Cache fields use COALESCE-with-NULL
        // semantics: pass NULL to fall back to the schema default
        // (cache_enabled=false, threshold=0.95, ttl=3600s).
        sqlx::query(
            "INSERT INTO services (name, service_type, description, strategy, guardrails, user_id, planner_model_id, system_prompt, max_iterations, aliases, cache_enabled, cache_similarity_threshold, cache_ttl_seconds)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, COALESCE($11, false), COALESCE($12, 0.95), COALESCE($13, 3600))"
        )
            .bind(&req.name)
            .bind(&req.service_type)
            .bind(&req.description)
            .bind(&strategy)
            .bind(&guardrails_json)
            .bind(user_id)
            .bind(&req.planner_model_id)
            .bind(&req.system_prompt)
            .bind(req.max_iterations.map(|i| i as i64))
            .bind(&req.aliases)
            .bind(req.cache_enabled)
            .bind(req.cache_similarity_threshold)
            .bind(req.cache_ttl_seconds)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database insert error: {}", e);
                // Postgres returns "unique_violation" (SQLSTATE 23505) when
                // the alias-uniqueness trigger fires. Surface that as 409
                // with the trigger's helpful message intact, in the
                // OpenAI envelope shape (#84).
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.code().as_deref() == Some("23505") {
                        return crate::openai_err::conflict(
                            db_err.message().to_string(),
                            Some("alias_collision"),
                        );
                    }
                }
                crate::openai_err::internal(format!("Failed to create service: {}", e))
            })?;

        let result = self.fetch_full_service(&req.name).await?;

        // Audit (#80): record the create with the after-state. Captures
        // IP + user agent so an operator can later answer "where did
        // this come from?" for compliance reviews.
        let (ip, ua) = mawi_core::audit::forensic_from_request(poem_req);
        mawi_core::audit::emit(
            self.pool.clone(),
            mawi_core::audit::action::SERVICE_CREATE,
            mawi_core::audit::resource("service", &req.name),
            mawi_core::audit::AuditContext {
                user_id: Some(user_id.clone()),
                ip_address: ip,
                user_agent: ua,
                after: serde_json::to_value(&result).ok(),
                ..Default::default()
            },
        );

        Ok(Json(result))
    }

    /// Update service
    /// Update a service. All fields in the request body are optional —
    /// only the ones present get applied; everything else is left as-is.
    /// Pass `aliases: []` to clear the alias list; omit `aliases` (or
    /// pass null) to leave it unchanged.
    ///
    /// Auth: caller must be the service's owner (`services.user_id`).
    /// System-level services (where `user_id IS NULL`) are read-only at
    /// this endpoint — they're managed via the YAML config loader.
    #[oai(path = "/services/:name", method = "put", tag = "ApiTags::Services")]
    async fn update_service(
        &self,
        name: Path<String>,
        req: Json<UpdateService>,
        poem_req: &poem::Request,
    ) -> poem::Result<Json<Service>> {
        // Auth gate: who is calling?
        let user = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .ok_or_else(|| crate::openai_err::unauthorized("Authentication required"))?;
        let user_id = &user.id;

        // Existence + ownership check in one query. Distinguish:
        //   row missing → 404
        //   row present but user_id is NULL (system service) → 403
        //   row present but user_id != caller → 403
        let owner: Option<Option<String>> =
            sqlx::query_scalar("SELECT user_id FROM services WHERE name = $1")
                .bind(&name.0)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    crate::openai_err::internal(format!("ownership lookup: {}", e))
                })?;
        match owner {
            None => {
                return Err(crate::openai_err::not_found(
                    format!("Service '{}' not found", name.0),
                    Some("service"),
                ));
            }
            Some(None) => {
                return Err(crate::openai_err::forbidden(
                    "Service is system-managed and cannot be updated via the API",
                ));
            }
            Some(Some(uid)) if &uid != user_id => {
                return Err(crate::openai_err::forbidden(
                    "Service belongs to a different user",
                ));
            }
            Some(Some(_)) => { /* owner matches — proceed */ }
        }

        // COALESCE pattern: bind Option<T> directly and let NULL mean
        // "leave the column alone." Single statement, atomic.
        // - service_type/description/strategy/pool_type/planner_model_id/
        //   system_prompt: Option<String>
        // - guardrails: Option<Vec<String>> → Option<String> via JSON
        //   (column is TEXT, see migration 001)
        // - max_iterations: Option<u32> → Option<i64> for Postgres int8
        // - aliases: Option<Vec<String>> → bound directly as TEXT[]; the
        //   trigger from migration 033 rejects collisions with 23505,
        //   which we map to 409 below.
        let guardrails_json = req
            .guardrails
            .as_ref()
            .map(|g| serde_json::to_string(g).unwrap_or_else(|_| "[]".to_string()));
        let max_iter = req.max_iterations.map(|i| i as i64);

        let result = sqlx::query(
            "UPDATE services SET
                service_type               = COALESCE($1, service_type),
                description                = COALESCE($2, description),
                strategy                   = COALESCE($3, strategy),
                guardrails                 = COALESCE($4, guardrails),
                pool_type                  = COALESCE($5, pool_type),
                planner_model_id           = COALESCE($6, planner_model_id),
                system_prompt              = COALESCE($7, system_prompt),
                max_iterations             = COALESCE($8, max_iterations),
                aliases                    = COALESCE($9, aliases),
                cache_enabled              = COALESCE($10, cache_enabled),
                cache_similarity_threshold = COALESCE($11, cache_similarity_threshold),
                cache_ttl_seconds          = COALESCE($12, cache_ttl_seconds)
            WHERE name = $13 AND user_id = $14",
        )
        .bind(&req.service_type)
        .bind(&req.description)
        .bind(&req.strategy)
        .bind(&guardrails_json)
        .bind(&req.pool_type)
        .bind(&req.planner_model_id)
        .bind(&req.system_prompt)
        .bind(max_iter)
        .bind(&req.aliases)
        .bind(req.cache_enabled)
        .bind(req.cache_similarity_threshold)
        .bind(req.cache_ttl_seconds)
        .bind(&name.0)
        .bind(user_id)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            // Map alias-collision (SQLSTATE 23505 from the trigger in
            // migration 033) to 409 with the trigger's message preserved
            // in the OpenAI envelope shape (#84).
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.code().as_deref() == Some("23505") {
                    return Err(crate::openai_err::conflict(
                        db_err.message().to_string(),
                        Some("alias_collision"),
                    ));
                }
            }
            return Err(crate::openai_err::internal(format!(
                "Failed to update service: {}",
                e
            )));
        }

        let result = self.fetch_full_service(&name.0).await?;

        // Audit (#80): emit with after-state. before-state capture is
        // a follow-up — would require a fetch *before* the UPDATE,
        // which doubles the DB round-trip on every service edit.
        // Without it, operators still see "user X updated service
        // text-default at 14:22 with this end state."
        let (ip, ua) = mawi_core::audit::forensic_from_request(poem_req);
        mawi_core::audit::emit(
            self.pool.clone(),
            mawi_core::audit::action::SERVICE_UPDATE,
            mawi_core::audit::resource("service", &name.0),
            mawi_core::audit::AuditContext {
                user_id: Some(user_id.clone()),
                ip_address: ip,
                user_agent: ua,
                after: serde_json::to_value(&result).ok(),
                ..Default::default()
            },
        );

        Ok(Json(result))
    }

    /// Delete service
    #[oai(path = "/services/:name", method = "delete", tag = "ApiTags::Services")]
    async fn delete_service(
        &self,
        name: Path<String>,
        poem_req: &poem::Request,
    ) -> poem::Result<Json<String>> {
        // Extract user_id from session (injected by AuthMiddleware)
        let user = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .ok_or_else(|| {
                poem::error::Error::from_string(
                    "Authentication required",
                    poem::http::StatusCode::UNAUTHORIZED,
                )
            })?;
        let user_id = &user.id;

        // First delete request_logs referencing this service
        sqlx::query("DELETE FROM request_logs WHERE service_name = $1")
            .bind(&name.0)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Failed to delete request logs: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        // Then delete associated service_models
        sqlx::query("DELETE FROM service_models WHERE service_name = $1")
            .bind(&name.0)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Failed to delete service models: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        // Finally delete the service (only if owned by this user)
        let result = sqlx::query("DELETE FROM services WHERE name = $1 AND user_id = $2")
            .bind(&name.0)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Failed to delete: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        if result.rows_affected() == 0 {
            return Err(poem::error::Error::from_string(
                format!(
                    "Service '{}' not found or you don't have permission to delete it",
                    name.0
                ),
                poem::http::StatusCode::NOT_FOUND,
            ));
        }

        // Audit (#80): record the deletion. before-state would require
        // capturing the service row before the DELETE; for now we
        // record the name + user, which is enough for "who deleted
        // production text-default at 3am?"
        let (ip, ua) = mawi_core::audit::forensic_from_request(poem_req);
        mawi_core::audit::emit(
            self.pool.clone(),
            mawi_core::audit::action::SERVICE_DELETE,
            mawi_core::audit::resource("service", &name.0),
            mawi_core::audit::AuditContext {
                user_id: Some(user_id.clone()),
                ip_address: ip,
                user_agent: ua,
                ..Default::default()
            },
        );

        Ok(Json("Service deleted".to_string()))
    }

    /// Assign model to service (with modality validation and weight)
    #[oai(
        path = "/services/:name/models",
        method = "post",
        tag = "ApiTags::Services"
    )]
    async fn assign_model(
        &self,
        name: Path<String>,
        req: Json<AssignModel>,
    ) -> poem::Result<Json<String>> {
        // Get service
        let service: Option<Service> = sqlx::query_as(
            "SELECT name, service_type, description, strategy, guardrails, created_at FROM services WHERE name = $1"
        )
            .bind(&name.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| poem::error::Error::from_string(
                format!("Database error: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR
            ))?;

        let service = service.ok_or_else(|| {
            poem::error::Error::from_string(
                format!("Service '{}' not found", name.0),
                poem::http::StatusCode::NOT_FOUND,
            )
        })?;

        // Get model
        let model: Option<Model> = sqlx::query_as("SELECT * FROM models WHERE id = $1")
            .bind(&req.model_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Database error: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        let model = model.ok_or_else(|| {
            poem::error::Error::from_string(
                format!("Model '{}' not found", req.model_id),
                poem::http::StatusCode::NOT_FOUND,
            )
        })?;

        // Validate modality for legacy service types only
        // POOL and AGENTIC services can have mixed modalities
        if !matches!(service.service_type.as_str(), "POOL" | "AGENTIC") {
            let expected_modality = match service.service_type.as_str() {
                "chat" => "text",
                "audio" => "audio",
                "video" => "video",
                _ => {
                    return Err(poem::error::Error::from_string(
                        format!("Invalid service type: {}", service.service_type),
                        poem::http::StatusCode::BAD_REQUEST,
                    ))
                }
            };

            if model.modality != expected_modality {
                return Err(poem::error::Error::from_string(
                    format!(
                        "Modality mismatch: service '{}' (type: {}) requires {} models, but model '{}' is {}",
                        service.name, service.service_type, expected_modality, model.name, model.modality
                    ),
                    poem::http::StatusCode::BAD_REQUEST
                ));
            }
        }

        // Insert assignment with weight and RTCROS fields. Idempotent
        // upsert — the UI lets users re-drop the same model into a
        // service (or re-submit the form), and that should *update*
        // the existing row's weight/position/RTCROS fields instead of
        // failing with "duplicate key value violates unique constraint
        // service_models_pkey." `ON CONFLICT (service_name, model_id)`
        // matches the PK from the initial migration.
        sqlx::query(
            "INSERT INTO service_models (service_name, model_id, modality, position, weight,
             rtcros_role, rtcros_task, rtcros_context, rtcros_reasoning, rtcros_output, rtcros_stop)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (service_name, model_id) DO UPDATE SET
                modality        = EXCLUDED.modality,
                position        = EXCLUDED.position,
                weight          = EXCLUDED.weight,
                rtcros_role     = EXCLUDED.rtcros_role,
                rtcros_task     = EXCLUDED.rtcros_task,
                rtcros_context  = EXCLUDED.rtcros_context,
                rtcros_reasoning = EXCLUDED.rtcros_reasoning,
                rtcros_output   = EXCLUDED.rtcros_output,
                rtcros_stop     = EXCLUDED.rtcros_stop"
        )
            .bind(&name.0)
            .bind(&req.model_id)
            .bind(&req.modality)
            .bind(req.position)
            .bind(req.weight)
            .bind(&req.rtcros_role)
            .bind(&req.rtcros_task)
            .bind(&req.rtcros_context)
            .bind(&req.rtcros_reasoning)
            .bind(&req.rtcros_output)
            .bind(&req.rtcros_stop)
            .execute(&self.pool)
            .await
            .map_err(|e| poem::error::Error::from_string(
                format!("Failed to assign model: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR
            ))?;

        // (Removed: the previous code here tried to INSERT a model_health
        // row using `INSERT OR IGNORE` — SQLite syntax that Postgres
        // rejects — and against a column set (`status`, `success_rate`,
        // `total_requests`, `failed_requests`) that does NOT exist in
        // the model_health table per migration 006. The `let _ = ...`
        // silently swallowed the error on every model assignment.
        // Downstream readers (`executor.rs:710,1343`) already handle
        // missing rows via `fetch_optional`, and `health.rs` creates a
        // row when it probes the model. So no replacement is needed.)

        // Check if total weight exceeds 100 and redistribute if needed
        let total_weight: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(weight), 0) FROM service_models WHERE service_name = $1",
        )
        .bind(&name.0)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if total_weight > 100 {
            // Auto-redistribute weights proportionally to maintain total = 100
            let scale_factor = 100.0 / total_weight as f64;

            // Get all models for this service
            let models: Vec<(String, i32)> = sqlx::query_as(
                "SELECT model_id, weight FROM service_models WHERE service_name = $1",
            )
            .bind(&name.0)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            // Update each model with proportional weight
            for (model_id, old_weight) in models {
                let new_weight = ((old_weight as f64 * scale_factor).round() as i32).max(1);
                let _ = sqlx::query(
                    "UPDATE service_models SET weight = $1 WHERE service_name = $2 AND model_id = $3"
                )
                    .bind(new_weight)
                    .bind(&name.0)
                    .bind(&model_id)
                    .execute(&self.pool)
                    .await;
            }

            eprintln!(
                "Auto-redistributed weights for service '{}': total was {}, scaled to 100",
                name.0, total_weight
            );
        }

        // Auto-detect and update service input/output modalities from assigned models
        self.update_service_capabilities(&name.0).await?;

        // REORDER POSITIONS BY WEIGHT
        self.reorder_service_models_by_weight(&name.0).await?;

        Ok(Json(format!(
            "Model '{}' assigned to service and weights auto-balanced",
            model.name
        )))
    }

    /// Get models assigned to a service
    #[oai(
        path = "/services/:name/models",
        method = "get",
        tag = "ApiTags::Services"
    )]
    async fn get_service_models(
        &self,
        name: Path<String>,
    ) -> poem::Result<Json<Vec<serde_json::Value>>> {
        #[derive(sqlx::FromRow)]
        struct ServiceModel {
            model_id: String,
            model_name: String,
            modality: String,
            position: i32,
            weight: i32,
            rtcros_role: Option<String>,
            rtcros_task: Option<String>,
            rtcros_context: Option<String>,
            rtcros_reasoning: Option<String>,
            rtcros_output: Option<String>,
            rtcros_stop: Option<String>,
            is_healthy: Option<bool>,
            last_error: Option<String>,
        }

        let models = sqlx::query_as::<_, ServiceModel>(
            "SELECT sm.model_id, m.name as model_name, sm.modality, sm.position, sm.weight,
             sm.rtcros_role, sm.rtcros_task, sm.rtcros_context, sm.rtcros_reasoning, sm.rtcros_output, sm.rtcros_stop,
             h.is_healthy, h.last_error
             FROM service_models sm
             JOIN models m ON sm.model_id = m.id
             LEFT JOIN model_health h ON m.id = h.model_id
             WHERE sm.service_name = $1
             ORDER BY sm.position ASC"
        )
            .bind(&name.0)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| poem::error::Error::from_string(
                format!("Database error: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR
            ))?;

        let result: Vec<serde_json::Value> = models
            .iter()
            .map(|m| {
                // Determine detailed health status
                let health_status = if m.is_healthy.unwrap_or(true) {
                    "healthy"
                } else {
                    let err = m.last_error.as_deref().unwrap_or("");
                    if err.contains("Rate Limited")
                        || err.contains("429")
                        || err.contains("Client Error")
                    {
                        "warning"
                    } else {
                        "unhealthy"
                    }
                };

                serde_json::json!({
                    "model_id": m.model_id,
                    "model_name": m.model_name,
                    "modality": m.modality,
                    "position": m.position,
                    "weight": m.weight,
                    "is_healthy": m.is_healthy.unwrap_or(true), // Keep for backward compatibility
                    "health_status": health_status,
                    "last_error": m.last_error,
                    "rtcros": {
                        "role": m.rtcros_role,
                        "task": m.rtcros_task,
                        "context": m.rtcros_context,
                        "reasoning": m.rtcros_reasoning,
                        "output": m.rtcros_output,
                        "stop": m.rtcros_stop,
                    }
                })
            })
            .collect();

        Ok(Json(result))
    }

    /// Bulk update model assignments (weights, positions, rtcros) transactionally
    #[oai(
        path = "/services/:name/models-bulk",
        method = "put",
        tag = "ApiTags::Services"
    )]
    async fn bulk_update_models(
        &self,
        name: Path<String>,
        req: Json<mawi_core::services::BulkUpdateServiceModels>,
    ) -> poem::Result<Json<String>> {
        // Start transaction
        let mut tx = self.pool.begin().await.map_err(|e| {
            poem::error::Error::from_string(
                format!("Failed to start transaction: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        // Fetch existing models for validation
        let existing_models: Vec<(String, i32)> =
            sqlx::query_as("SELECT model_id, weight FROM service_models WHERE service_name = $1")
                .bind(&name.0)
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| {
                    poem::error::Error::from_string(
                        format!("DB Error: {}", e),
                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?;

        if existing_models.is_empty() {
            return Err(poem::error::Error::from_string(
                format!("No assignments found for service {}", name.0),
                poem::http::StatusCode::NOT_FOUND,
            ));
        }

        let mut final_weights = std::collections::HashMap::new();
        for (mid, w) in &existing_models {
            final_weights.insert(mid.clone(), *w);
        }

        // Overlay updates
        for update in &req.models {
            if let Some(w) = update.weight {
                final_weights.insert(update.model_id.clone(), w);
            }
        }

        // Check Sum
        let sum: i32 = final_weights.values().sum();
        if sum != 100 {
            return Err(poem::error::Error::from_string(
                format!("Total weight must sum to 100. Current sum: {}", sum),
                poem::http::StatusCode::BAD_REQUEST,
            ));
        }

        // Apply updates
        for update in &req.models {
            let mut updates_sql = Vec::new();
            let mut params: Vec<String> = Vec::new();
            let mut param_idx = 1;

            if let Some(pos) = update.position {
                updates_sql.push(format!("position = ${}", param_idx));
                param_idx += 1;
                params.push(pos.to_string());
            }
            if let Some(w) = update.weight {
                updates_sql.push(format!("weight = ${}", param_idx));
                param_idx += 1;
                params.push(w.to_string());
            }
            // RTCROS
            if let Some(ref v) = update.rtcros_role {
                updates_sql.push(format!("rtcros_role = ${}", param_idx));
                param_idx += 1;
                params.push(v.clone());
            }
            if let Some(ref v) = update.rtcros_task {
                updates_sql.push(format!("rtcros_task = ${}", param_idx));
                param_idx += 1;
                params.push(v.clone());
            }
            if let Some(ref v) = update.rtcros_context {
                updates_sql.push(format!("rtcros_context = ${}", param_idx));
                param_idx += 1;
                params.push(v.clone());
            }
            if let Some(ref v) = update.rtcros_reasoning {
                updates_sql.push(format!("rtcros_reasoning = ${}", param_idx));
                param_idx += 1;
                params.push(v.clone());
            }
            if let Some(ref v) = update.rtcros_output {
                updates_sql.push(format!("rtcros_output = ${}", param_idx));
                param_idx += 1;
                params.push(v.clone());
            }
            if let Some(ref v) = update.rtcros_stop {
                updates_sql.push(format!("rtcros_stop = ${}", param_idx));
                param_idx += 1;
                params.push(v.clone());
            }

            if !updates_sql.is_empty() {
                let query = format!(
                    "UPDATE service_models SET {} WHERE service_name = ${} AND model_id = ${}",
                    updates_sql.join(", "),
                    param_idx,
                    param_idx + 1
                );

                let mut q = sqlx::query(&query);
                for p in params {
                    q = q.bind(p);
                }
                q = q.bind(&name.0).bind(&update.model_id);

                q.execute(&mut *tx).await.map_err(|e| {
                    poem::error::Error::from_string(
                        format!("Failed to update model {}: {}", update.model_id, e),
                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?;
            }
        }

        tx.commit().await.map_err(|e| {
            poem::error::Error::from_string(
                format!("Failed to commit transaction: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        Ok(Json("Bulk update successful".to_string()))
    }

    /// Update model assignment (weight, position, RTCROS)
    #[oai(path = "/services/:name/models/:model_id", method = "put")]
    async fn update_model_assignment(
        &self,
        name: Path<String>,
        model_id: Path<String>,
        req: Json<UpdateModelAssignment>,
    ) -> poem::Result<Json<String>> {
        // Check if assignment exists
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM service_models WHERE service_name = $1 AND model_id = $2",
        )
        .bind(&name.0)
        .bind(&model_id.0)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0)
            > 0;

        if !exists {
            return Err(poem::error::Error::from_string(
                format!(
                    "Model assignment not found (service='{}', model='{}')",
                    name.0, model_id.0
                ),
                poem::http::StatusCode::NOT_FOUND,
            ));
        }

        let mut updates = Vec::new();
        let mut params: Vec<String> = Vec::new();
        let mut param_idx = 1;

        // Standard fields
        if let Some(pos) = req.position {
            updates.push(format!("position = ${}", param_idx));
            param_idx += 1;
            params.push(pos.to_string());
        }
        if let Some(w) = req.weight {
            // Validate new total weight
            let current_total: i64 = sqlx::query_scalar(
                "SELECT COALESCE(SUM(weight), 0) FROM service_models WHERE service_name = $1",
            )
            .bind(&name.0)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

            let old_weight: i64 = sqlx::query_scalar(
                "SELECT weight FROM service_models WHERE service_name = $1 AND model_id = $2",
            )
            .bind(&name.0)
            .bind(&model_id.0)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

            if current_total - old_weight + w as i64 > 100
                && (current_total - old_weight + w as i64 >= current_total)
            {
                return Err(poem::error::Error::from_string(
                    format!("Total weight for service '{}' cannot exceed 100 (unless reducing existing total). Current total: {}, Resulting total: {}", 
                            name.0, current_total, current_total - old_weight + w as i64),
                    poem::http::StatusCode::BAD_REQUEST
                ));
            }

            updates.push(format!("weight = ${}", param_idx));
            param_idx += 1;
            params.push(w.to_string());
        }

        // RTCROS fields
        if let Some(ref role) = req.rtcros_role {
            updates.push(format!("rtcros_role = ${}", param_idx));
            param_idx += 1;
            params.push(role.clone());
        }
        if let Some(ref task) = req.rtcros_task {
            updates.push(format!("rtcros_task = ${}", param_idx));
            param_idx += 1;
            params.push(task.clone());
        }
        if let Some(ref context) = req.rtcros_context {
            updates.push(format!("rtcros_context = ${}", param_idx));
            param_idx += 1;
            params.push(context.clone());
        }
        if let Some(ref reasoning) = req.rtcros_reasoning {
            updates.push(format!("rtcros_reasoning = ${}", param_idx));
            param_idx += 1;
            params.push(reasoning.clone());
        }
        if let Some(ref output) = req.rtcros_output {
            updates.push(format!("rtcros_output = ${}", param_idx));
            param_idx += 1;
            params.push(output.clone());
        }
        if let Some(ref stop) = req.rtcros_stop {
            updates.push(format!("rtcros_stop = ${}", param_idx));
            param_idx += 1;
            params.push(stop.clone());
        }

        if !updates.is_empty() {
            let query = format!(
                "UPDATE service_models SET {} WHERE service_name = ${} AND model_id = ${}",
                updates.join(", "),
                param_idx,
                param_idx + 1
            );

            let mut q = sqlx::query(&query);
            for param in params {
                q = q.bind(param);
            }
            q = q.bind(&name.0);
            q.bind(&model_id.0).execute(&self.pool).await.map_err(|e| {
                poem::error::Error::from_string(
                    format!("Failed to update assignment: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;
        }

        // REORDER POSITIONS BY WEIGHT if weight was updated
        if req.weight.is_some() {
            self.reorder_service_models_by_weight(&name.0).await?;
        }

        Ok(Json(
            "Model assignment updated and positions reordered by weight".to_string(),
        ))
    }

    /// Remove model from service
    #[oai(path = "/services/:name/models/:model_id", method = "delete")]
    async fn remove_model_from_service(
        &self,
        name: Path<String>,
        model_id: Path<String>,
    ) -> poem::Result<Json<String>> {
        let result =
            sqlx::query("DELETE FROM service_models WHERE service_name = $1 AND model_id = $2")
                .bind(&name.0)
                .bind(&model_id.0)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    poem::error::Error::from_string(
                        format!("Database error: {}", e),
                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?;

        if result.rows_affected() == 0 {
            return Err(poem::error::Error::from_string(
                format!(
                    "Model '{}' is not assigned to service '{}'",
                    model_id.0, name.0
                ),
                poem::http::StatusCode::NOT_FOUND,
            ));
        }

        // Auto-update service capabilities after model removal
        self.update_service_capabilities(&name.0).await?;

        // Redistribute weights for remaining models to sum to 100%
        let remaining_models: Vec<(String, i32)> =
            sqlx::query_as("SELECT model_id, weight FROM service_models WHERE service_name = $1")
                .bind(&name.0)
                .fetch_all(&self.pool)
                .await
                .unwrap_or_default();

        if !remaining_models.is_empty() {
            let total_weight: i64 = remaining_models.iter().map(|(_, w)| *w as i64).sum();

            // Only redistribute if total is not already 100
            if total_weight != 100 && total_weight > 0 {
                let scale_factor = 100.0 / total_weight as f64;

                eprintln!(
                    "Redistributing weights for service '{}' after model removal: total was {}, scaling to 100",
                    name.0, total_weight
                );

                // Update each model with proportional weight
                for (remaining_model_id, old_weight) in remaining_models {
                    let new_weight = ((old_weight as f64 * scale_factor).round() as i32).max(1);
                    let _ = sqlx::query(
                        "UPDATE service_models SET weight = $1 WHERE service_name = $2 AND model_id = $3"
                    )
                        .bind(new_weight)
                        .bind(&name.0)
                        .bind(&remaining_model_id)
                        .execute(&self.pool)
                        .await;
                }

                // Reorder positions by weight
                self.reorder_service_models_by_weight(&name.0).await?;
            }
        }

        Ok(Json(format!(
            "Model '{}' removed from service and weights auto-balanced",
            model_id.0
        )))
    }

    /// Helper: Auto-detect and update service input/output modalities from assigned models
    async fn update_service_capabilities(&self, service_name: &str) -> poem::Result<()> {
        use std::collections::HashSet;

        // Query all models assigned to this service with their modality
        let models: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT m.modality 
             FROM service_models sm
             JOIN models m ON sm.model_id = m.id
             WHERE sm.service_name = $1
             AND m.modality IS NOT NULL",
        )
        .bind(service_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            poem::error::Error::from_string(
                format!("Failed to query service models: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        let mut input_set = HashSet::new();
        let mut output_set = HashSet::new();

        for mod_str in models {
            let m = mod_str.to_lowercase();
            if m.contains("text") {
                input_set.insert("text".to_string());
                output_set.insert("text".to_string());
            } else if m.contains("image") {
                input_set.insert("text".to_string()); // Prompt
                output_set.insert("image".to_string());
            } else if m.contains("video") {
                input_set.insert("text".to_string()); // Prompt
                output_set.insert("video".to_string());
            } else if m.contains("audio") {
                input_set.insert("text".to_string());
                output_set.insert("audio".to_string());
                input_set.insert("audio".to_string()); // Support both directions slightly ambiguously to be safe
            } else {
                // Fallback
                input_set.insert(m.clone());
                output_set.insert(m);
            }
        }

        // Convert to JSON array
        let to_json = |set: HashSet<String>| -> Option<String> {
            if set.is_empty() {
                None
            } else {
                let mut sorted: Vec<String> = set.into_iter().collect();
                sorted.sort();
                Some(serde_json::to_string(&sorted).unwrap_or_else(|_| "[]".to_string()))
            }
        };

        sqlx::query(
            "UPDATE services 
             SET input_modalities = $1, output_modalities = $2
             WHERE name = $3",
        )
        .bind(to_json(input_set))
        .bind(to_json(output_set))
        .bind(service_name)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            poem::error::Error::from_string(
                format!("Failed to update service capabilities: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        Ok(())
    }

    /// Helper: Reorder service models by weight descending
    async fn reorder_service_models_by_weight(&self, service_name: &str) -> poem::Result<()> {
        #[derive(sqlx::FromRow)]
        struct WeightModel {
            model_id: String,
            weight: i32,
        }

        let models = sqlx::query_as::<_, WeightModel>(
            "SELECT model_id, weight FROM service_models WHERE service_name = $1 ORDER BY weight DESC, position ASC"
        )
            .bind(service_name)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| poem::error::Error::from_string(
                format!("Failed to query service models for reordering: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR
            ))?;

        eprintln!(
            "Reordering models for service '{}' (count: {})",
            service_name,
            models.len()
        );

        for (i, row) in models.into_iter().enumerate() {
            let position = (i + 1) as i32;
            eprintln!(
                "  → Setting model '{}' to position {} (weight: {})",
                row.model_id, position, row.weight
            );
            sqlx::query(
                "UPDATE service_models SET position = $1 WHERE service_name = $2 AND model_id = $3",
            )
            .bind(position)
            .bind(service_name)
            .bind(&row.model_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Failed to update model position: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;
        }

        Ok(())
    }

    // ==================== AGENTIC TOOLS ====================

    /// Add a tool to an agentic service
    #[oai(
        path = "/services/:name/tools",
        method = "post",
        tag = "ApiTags::Services"
    )]
    async fn add_service_tool(
        &self,
        name: Path<String>,
        req: Json<mawi_core::tools::CreateTool>,
    ) -> poem::Result<Json<String>> {
        // Verify service exists and is AGENTIC
        let service =
            sqlx::query_as::<_, (String,)>("SELECT service_type FROM services WHERE name = $1")
                .bind(&name.0)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    poem::error::Error::from_string(
                        format!("Database error: {}", e),
                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?
                .ok_or_else(|| {
                    poem::error::Error::from_string(
                        format!("Service '{}' not found", name.0),
                        poem::http::StatusCode::NOT_FOUND,
                    )
                })?;

        if service.0 != "AGENTIC" {
            return Err(poem::error::Error::from_string(
                format!("Service '{}' is not an AGENTIC service", name.0),
                poem::http::StatusCode::BAD_REQUEST,
            ));
        }

        // Insert tool
        let tool_id = Uuid::new_v4().to_string();
        let params_schema = req
            .parameters_schema
            .as_ref()
            .map(|s| serde_json::to_string(s).unwrap_or_default());

        sqlx::query(
            "INSERT INTO agentic_tools (id, service_name, name, description, tool_type, target_id, parameters_schema, position)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(&tool_id)
        .bind(&name.0)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.tool_type)
        .bind(&req.target_id)
        .bind(&params_schema)
        .bind(req.position)
        .execute(&self.pool)
        .await
        .map_err(|e| poem::error::Error::from_string(
            format!("Failed to add tool: {}", e),
            poem::http::StatusCode::INTERNAL_SERVER_ERROR
        ))?;

        Ok(Json(format!("Tool '{}' added to service", req.name)))
    }

    /// List tools for an agentic service
    #[oai(
        path = "/services/:name/tools",
        method = "get",
        tag = "ApiTags::Services"
    )]
    async fn list_service_tools(
        &self,
        name: Path<String>,
    ) -> poem::Result<Json<Vec<serde_json::Value>>> {
        #[derive(sqlx::FromRow)]
        struct ToolRow {
            id: String,
            name: String,
            description: String,
            tool_type: String,
            target_id: String,
            parameters_schema: Option<String>,
            position: i32,
        }

        let tools = sqlx::query_as::<_, ToolRow>(
            "SELECT id, name, description, tool_type, target_id, parameters_schema, position
             FROM agentic_tools
             WHERE service_name = $1
             ORDER BY position ASC",
        )
        .bind(&name.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            poem::error::Error::from_string(
                format!("Database error: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        let result: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "name": t.name,
                    "description": t.description,
                    "tool_type": t.tool_type,
                    "target_id": t.target_id,
                    "parameters_schema": t.parameters_schema.as_ref()
                        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
                    "position": t.position,
                })
            })
            .collect();

        Ok(Json(result))
    }

    /// Delete a tool from an agentic service
    #[oai(
        path = "/services/:name/tools/:tool_id",
        method = "delete",
        tag = "ApiTags::Services"
    )]
    async fn delete_service_tool(
        &self,
        name: Path<String>,
        tool_id: Path<String>,
    ) -> poem::Result<Json<String>> {
        let result = sqlx::query("DELETE FROM agentic_tools WHERE id = $1 AND service_name = $2")
            .bind(&tool_id.0)
            .bind(&name.0)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                poem::error::Error::from_string(
                    format!("Failed to delete tool: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        if result.rows_affected() == 0 {
            return Err(poem::error::Error::from_string(
                format!("Tool '{}' not found for service '{}'", tool_id.0, name.0),
                poem::http::StatusCode::NOT_FOUND,
            ));
        }

        Ok(Json("Tool deleted".to_string()))
    }

    // ==================== SERVICE MCP SERVERS ====================

    /// List MCP servers assigned to a service
    #[oai(
        path = "/services/:name/mcp-servers",
        method = "get",
        tag = "ApiTags::Services"
    )]
    async fn list_service_mcp_servers(
        &self,
        name: Path<String>,
    ) -> poem::Result<Json<Vec<serde_json::Value>>> {
        #[derive(sqlx::FromRow)]
        struct McpServerRow {
            id: String,
            name: String,
            server_type: String,
            status: String,
            image_or_command: String,
        }

        let servers = sqlx::query_as::<_, McpServerRow>(
            "SELECT ms.id, ms.name, ms.server_type, ms.status, ms.image_or_command
             FROM mcp_servers ms
             JOIN service_mcp_servers sms ON sms.mcp_server_id = ms.id
             WHERE sms.service_name = $1
             ORDER BY ms.name",
        )
        .bind(&name.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            poem::error::Error::from_string(
                format!("Database error: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        let result: Vec<serde_json::Value> = servers
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "name": s.name,
                    "server_type": s.server_type,
                    "status": s.status,
                    "image_or_command": s.image_or_command,
                })
            })
            .collect();

        Ok(Json(result))
    }

    /// Assign MCP server to a service
    #[oai(
        path = "/services/:name/mcp-servers/:server_id",
        method = "post",
        tag = "ApiTags::Services"
    )]
    async fn assign_mcp_server(
        &self,
        name: Path<String>,
        server_id: Path<String>,
    ) -> poem::Result<Json<String>> {
        // Verify service exists and is AGENTIC
        let service =
            sqlx::query_as::<_, (String,)>("SELECT service_type FROM services WHERE name = $1")
                .bind(&name.0)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    poem::error::Error::from_string(
                        format!("Database error: {}", e),
                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?
                .ok_or_else(|| {
                    poem::error::Error::from_string(
                        format!("Service '{}' not found", name.0),
                        poem::http::StatusCode::NOT_FOUND,
                    )
                })?;

        if service.0 != "AGENTIC" {
            return Err(poem::error::Error::from_string(
                format!("Service '{}' is not an AGENTIC service. MCP servers can only be assigned to AGENTIC services.", name.0),
                poem::http::StatusCode::BAD_REQUEST
            ));
        }

        // Verify MCP server exists
        let server_exists =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM mcp_servers WHERE id = $1")
                .bind(&server_id.0)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0)
                > 0;

        if !server_exists {
            return Err(poem::error::Error::from_string(
                format!("MCP server '{}' not found", server_id.0),
                poem::http::StatusCode::NOT_FOUND,
            ));
        }

        // Insert assignment (ignore if already exists)
        sqlx::query(
            "INSERT INTO service_mcp_servers (service_name, mcp_server_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
        )
        .bind(&name.0)
        .bind(&server_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| poem::error::Error::from_string(
            format!("Failed to assign MCP server: {}", e),
            poem::http::StatusCode::INTERNAL_SERVER_ERROR
        ))?;

        eprintln!(
            "Assigned MCP server '{}' to service '{}'",
            server_id.0, name.0
        );

        Ok(Json("MCP server assigned to service".to_string()))
    }

    /// Remove MCP server from a service
    #[oai(
        path = "/services/:name/mcp-servers/:server_id",
        method = "delete",
        tag = "ApiTags::Services"
    )]
    async fn remove_mcp_server(
        &self,
        name: Path<String>,
        server_id: Path<String>,
    ) -> poem::Result<Json<String>> {
        let result = sqlx::query(
            "DELETE FROM service_mcp_servers WHERE service_name = $1 AND mcp_server_id = $2",
        )
        .bind(&name.0)
        .bind(&server_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            poem::error::Error::from_string(
                format!("Failed to remove MCP server: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        if result.rows_affected() == 0 {
            return Err(poem::error::Error::from_string(
                format!(
                    "MCP server '{}' is not assigned to service '{}'",
                    server_id.0, name.0
                ),
                poem::http::StatusCode::NOT_FOUND,
            ));
        }

        eprintln!(
            "Removed MCP server '{}' from service '{}'",
            server_id.0, name.0
        );

        Ok(Json("MCP server removed from service".to_string()))
    }
}
