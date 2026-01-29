use sqlx::PgPool;
use std::time::{Duration, Instant};
use tokio::time::interval;
use anyhow::Result;
use poem::{handler, IntoResponse};
use futures::{stream, StreamExt};
use std::sync::Arc;

#[handler]
pub fn health_check() -> impl IntoResponse {
    "OK"
}

#[allow(dead_code)]
pub struct HealthMonitor {
    pool: PgPool,
}

#[allow(dead_code)]
impl HealthMonitor {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Start background health monitoring
    pub fn start(pool: PgPool) {
        tokio::spawn(async move {
            let monitor = Self::new(pool);
            let mut ticker = interval(Duration::from_secs(300)); // Check every 5 minutes

            eprintln!("🏥 Health monitor started - checking every 5 minutes");

            loop {
                ticker.tick().await;
                if let Err(e) = monitor.check_all_models().await {
                    eprintln!("❌ Health check error: {}", e);
                }
            }
        });
    }

    /// Check health of all registered models
    async fn check_all_models(&self) -> Result<()> {
        let models_vec = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, name, provider_id, modality FROM models"
        )
        .fetch_all(&self.pool)
        .await?;

        eprintln!("🔍 Health check: scanning {} models", models_vec.len());

        // Parallelize checks (Limit concurrency to 10 to avoid FD exhaustion)
        stream::iter(models_vec)
            .for_each_concurrent(Some(10), |(model_id, model_name, provider_id, modality)| {
                // Clone pool for the task
                let pool = self.pool.clone();
                let monitor = Self::new(pool);
                
                async move {
                    let health = monitor.ping_single_model(&model_id, &model_name, &provider_id, &modality).await;
                    
                    // Update health table  
                    let update_result = sqlx::query(
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
                    .bind(&model_id)
                    .bind(health.is_healthy as i64)
                    .bind(chrono::Utc::now().timestamp())
                    .bind(health.response_time_ms)
                    .bind(health.consecutive_failures)
                    .bind(&health.last_error)
                    .execute(&monitor.pool)
                    .await;

                    if let Err(e) = update_result {
                        eprintln!("❌ Failed to update health for {}: {}", model_name, e);
                    } else {
                        let status = if health.is_healthy { "✅" } else { "❌" };
                        // Only log errors or slow responses to reduce noise? No, keep existing behavior.
                        eprintln!("{} {} - {}ms", status, model_name, health.response_time_ms.unwrap_or(0));
                    }
                }
            })
            .await;

        Ok(())
    }

    /// Ping a specific model with a lightweight request (public for manual checks)
    pub async fn ping_single_model(&self, model_id: &str, model_name: &str, provider_id: &str, modality: &str) -> HealthStatus {
        if modality == "image" {
            // Skip expensive image generation health checks for now
            // TODO: Implement lightweight health check for image models
            return HealthStatus {
                is_healthy: true,
                response_time_ms: Some(0),
                consecutive_failures: 0,
                last_error: None,
            };
        }

        let start = Instant::now();

        // Get provider details including endpoint
        let provider = match sqlx::query_as::<_, (String, Option<String>, String)>(
            "SELECT provider_type, api_endpoint, api_key FROM providers WHERE id = $1"
        )
        .bind(provider_id)
        .fetch_one(&self.pool)
        .await
        {
            Ok(p) => p,
            Err(e) => {
                return HealthStatus {
                    is_healthy: false,
                    response_time_ms: None,
                    consecutive_failures: 1,
                    last_error: Some(format!("Provider not found: {}", e)),
                };
            }
        };

        let (provider_type, api_endpoint, api_key) = provider;

        // Determine endpoint/key based on provider type
        let (endpoint, final_api_key) = match provider_type.as_str() {
            "openai" => ("https://api.openai.com/v1".to_string(), api_key),
            "google" | "gemini" => ("https://generativelanguage.googleapis.com/v1beta/models".to_string(), api_key),
            "anthropic" => ("https://api.anthropic.com/v1".to_string(), api_key),
            "xai" => ("https://api.x.ai/v1".to_string(), api_key),
            "mistral" => ("https://api.mistral.ai/v1".to_string(), api_key),
            "azure" => {
                // Fetch model-specific overrides for Azure
                let model_overrides = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>)>(
                    "SELECT api_endpoint, api_key, api_version FROM models WHERE id = $1"
                )
                .bind(model_id)
                .fetch_optional(&self.pool)
                .await
                .ok()
                .flatten();

                let (m_endpoint, m_key, _m_ver) = model_overrides.unwrap_or((None, None, None));
                
                let ep = m_endpoint.or(api_endpoint).unwrap_or_else(|| {
                   eprintln!("⚠️  Azure provider missing api_endpoint");
                   "".to_string()
                });
                
                let key = m_key.unwrap_or(api_key);
                (ep, key)
            },
            _ => {
                 return HealthStatus {
                    is_healthy: false,
                    response_time_ms: None,
                    consecutive_failures: 1,
                    last_error: Some(format!("Unknown provider type: {}", provider_type)),
                 };
            }
        };

        // Simple health check request based on provider type - use model_name not model_id
        let result = match provider_type.as_str() {
            "openai" => self.ping_openai(&endpoint, &final_api_key, model_name).await,
            "google" | "gemini" => self.ping_gemini(&endpoint, &final_api_key, model_name).await,
            "azure" => self.ping_azure(&endpoint, &final_api_key, model_name).await,
            "anthropic" => self.ping_anthropic(&endpoint, &final_api_key, model_name).await,
            "xai" => self.ping_openai(&endpoint, &final_api_key, model_name).await,
            "mistral" => self.ping_openai(&endpoint, &final_api_key, model_name).await,
            _ => Err(anyhow::anyhow!("Unknown provider type")),
        };

        let latency = start.elapsed().as_millis() as i64;

        match result {
            Ok(_) => HealthStatus {
                is_healthy: true,
                response_time_ms: Some(latency),
                consecutive_failures: 0,
                last_error: None,
            },
            Err(e) => HealthStatus {
                is_healthy: false,
                response_time_ms: Some(latency),
                consecutive_failures: 1,
                last_error: Some(e.to_string()),
            },
        }
    }

    /// Ping OpenAI endpoint
    async fn ping_openai(&self, endpoint: &str, api_key: &str, model: &str) -> Result<()> {
        let client = reqwest::Client::new();
        
        let response: reqwest::Response = client
            .post(format!("{}/chat/completions", endpoint))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": "ping"}],
                "max_tokens": 1
            }))
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Err(anyhow::anyhow!("Rate Limited (429)"))
        } else if status.is_client_error() {
            Err(anyhow::anyhow!("Client Error ({})", status))
        } else {
            Err(anyhow::anyhow!("Server Error ({})", status))
        }
    }

    /// Ping Gemini endpoint
    async fn ping_gemini(&self, endpoint: &str, api_key: &str, model: &str) -> Result<()> {
        let client = reqwest::Client::new();
        
        let response: reqwest::Response = client
            .post(format!("{}/{}:generateContent?key={}", endpoint, model, api_key))
            .json(&serde_json::json!({
                "contents": [{
                    "parts": [{"text": "ping"}]
                }],
                "generationConfig": {
                    "maxOutputTokens": 1
                }
            }))
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Err(anyhow::anyhow!("Rate Limited (429)"))
        } else if status.is_client_error() {
            Err(anyhow::anyhow!("Client Error ({})", status))
        } else {
            Err(anyhow::anyhow!("Server Error ({})", status))
        }
    }

    /// Ping Anthropic endpoint
    async fn ping_anthropic(&self, endpoint: &str, api_key: &str, model: &str) -> Result<()> {
        let client = reqwest::Client::new();
        
        let response: reqwest::Response = client
            .post(format!("{}/messages", endpoint))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": "ping"}],
                "max_tokens": 1
            }))
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Err(anyhow::anyhow!("Rate Limited (429)"))
        } else if status.is_client_error() {
            Err(anyhow::anyhow!("Client Error ({})", status))
        } else {
            Err(anyhow::anyhow!("Server Error ({})", status))
        }
    }

    /// Health check for Azure OpenAI
    async fn ping_azure(&self, base_url: &str, api_key: &str, deployment: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let base_url = base_url.trim_end_matches('/');
        
        let response: reqwest::Response = client
            .post(format!("{}/openai/deployments/{}/chat/completions?api-version=2024-12-01-preview", base_url, deployment))
            .header("api-key", api_key)
            .json(&serde_json::json!({
                "messages": [{
                    "role": "user",
                    "content": "ping"
                }],
                "max_tokens": 1
            }))
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Err(anyhow::anyhow!("Rate Limited (429)"))
        } else if status.is_client_error() {
            Err(anyhow::anyhow!("Client Error ({})", status))
        } else {
            Err(anyhow::anyhow!("Server Error ({})", status))
        }
    }
}

#[allow(dead_code)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub response_time_ms: Option<i64>,
    pub consecutive_failures: i32,
    pub last_error: Option<String>,
}
