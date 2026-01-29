use poem_openapi::{payload::Json, OpenApi, Object, param::Path};
use sqlx::PgPool;
use mawi_core::models::{Model, Provider};
use mawi_core::services::Service;
use crate::api::{ProviderResponse, ApiTags};
use serde::Serialize;

#[derive(Debug, Serialize, Object)]
pub struct ServiceWithModels {
    pub service: Service,
    pub models: Vec<ServiceModelInfo>,
    pub mcp_servers: Vec<McpServerInfo>,
}

#[derive(Debug, Serialize, Object, sqlx::FromRow)]
pub struct ServiceModelInfo {
    pub model_id: String,
    pub model_name: String,
    pub position: i32,
    pub weight: Option<i32>,
    pub provider_id: String,
    pub modality: String,
    pub is_healthy: Option<bool>,
    pub health_status: Option<String>,
}

#[derive(Debug, Serialize, Object, sqlx::FromRow, Clone)]
pub struct McpServerInfo {
    pub id: String,
    pub name: String,
    pub server_type: String,
    pub status: String,
}

#[derive(Debug, Serialize, Object)]
pub struct TopologyResponse {
    pub providers: Vec<ProviderResponse>,
    pub services: Vec<ServiceWithModels>,
    pub models: Vec<Model>,
}

pub struct TopologyApi {
    pub pool: PgPool,
}

#[OpenApi]
impl TopologyApi {
    /// Get full system topology (Providers, Services, Models) - filtered by authenticated user
    #[oai(path = "/topology", method = "get", tag = "ApiTags::System")]
    async fn get_topology(&self, req: &poem::Request) -> poem::Result<Json<TopologyResponse>> {
        // Authenticate user
        use poem::http::header;
        
        let token = req.headers().get(header::COOKIE)
            .and_then(|c| c.to_str().ok())
            .and_then(|cookies| {
                for cookie in cookies.split(';') {
                    let cookie = cookie.trim();
                    if let Some(value) = cookie.strip_prefix("session_token=") {
                        return Some(value.to_string());
                    }
                }
                None
            });
        
        let user_id = match token {
            Some(t) => {
                let auth_service = mawi_core::auth::AuthService::new(self.pool.clone());
                match auth_service.validate_session(&t).await {
                    Ok(user) => user.id,
                    Err(_) => {
                        return Err(poem::Error::from_string(
                            "Unauthorized",
                            poem::http::StatusCode::UNAUTHORIZED
                        ));
                    }
                }
            },
            None => {
                return Err(poem::Error::from_string(
                    "Missing session",
                    poem::http::StatusCode::UNAUTHORIZED
                ));
            }
        };
        
        // Fetch user-specific data in parallel
        let (providers_res, services_res, models_res) = tokio::join!(
            sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE user_id = $1 ORDER BY name")
                .bind(&user_id)
                .fetch_all(&self.pool),
            sqlx::query_as::<_, Service>("SELECT * FROM services WHERE user_id = $1 ORDER BY name")
                .bind(&user_id)
                .fetch_all(&self.pool),
            sqlx::query_as::<_, Model>("SELECT * FROM models WHERE user_id = $1 ORDER BY name")
                .bind(&user_id)
                .fetch_all(&self.pool)
        );

        let providers = providers_res.unwrap_or_default();
        let services = services_res.unwrap_or_default();
        let all_models = models_res.unwrap_or_default();

        // For each service, fetch its assigned models (also filtered by user)
        let _all_service_models = sqlx::query_as::<_, (String, String, String, i64, String, String, Option<i32>, Option<bool>)>("
            SELECT sm.service_name, sm.model_id, sm.position, 
                   m.name as model_name, m.provider_id, m.modality,
                   sm.weight,
                   h.is_healthy
            FROM service_models sm
            JOIN models m ON sm.model_id = m.id
            JOIN services s ON sm.service_name = s.name
            LEFT JOIN model_health h ON m.id = h.model_id
            WHERE s.user_id = $1
            ORDER BY sm.service_name, sm.position
        ")
        .bind(&user_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();


        // Group models by service
        // We need to map the raw query to our struct
        // But sqlx tuple mapping is tricky with clean structs.
        // Let's iterate.

        let mut services_with_models = Vec::new();

        for service in services {
            // Find models for this service from the bulk fetch
            // This is O(N*M) but N and M are small.
            // Using a simpler per-service query might be cleaner code-wise and still fast because local network.
            // But let's stick to the N+1 elimination.
            
            let models = sqlx::query_as::<_, ServiceModelInfo>(
                "SELECT sm.model_id, m.name as model_name, sm.position,
                        sm.weight, m.provider_id, m.modality,
                        h.is_healthy, NULL as health_status
                 FROM service_models sm
                 JOIN models m ON sm.model_id = m.id
                 LEFT JOIN model_health h ON m.id = h.model_id
                 WHERE sm.service_name = $1
                 ORDER BY sm.position"
            )
            .bind(&service.name)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();
            
            // Calculate weights default if missing
             let models_with_weights = models.into_iter().map(|mut m| {
                if m.weight.is_none() {
                    let w = 100 - m.position.min(99); 
                    m.weight = Some(w);
                }
                // Health status string
                m.health_status = match m.is_healthy {
                    Some(true) => Some("healthy".to_string()),
                    Some(false) => Some("unhealthy".to_string()),
                    None => Some("unknown".to_string()),
                };
                m
            }).collect();

            // Fetch MCP servers assigned to this service
            let mcp_servers = sqlx::query_as::<_, McpServerInfo>(
                "SELECT ms.id, ms.name, ms.server_type, ms.status
                 FROM mcp_servers ms
                 JOIN service_mcp_servers sms ON sms.mcp_server_id = ms.id
                 WHERE sms.service_name = $1
                 ORDER BY ms.name"
            )
            .bind(&service.name)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            services_with_models.push(ServiceWithModels {
                service,
                models: models_with_weights,
                mcp_servers,
            });
        }

        Ok(Json(TopologyResponse {
            providers: providers.into_iter().map(ProviderResponse::from).collect(),
            services: services_with_models,
            models: all_models,
        }))
    }
}
