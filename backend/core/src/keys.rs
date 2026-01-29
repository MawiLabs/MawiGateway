use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// Virtual Keys for access control
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct VirtualKey {
    pub id: String,
    pub key_hash: String,
    pub name: Option<String>,
    pub service_filter: Option<String>, // JSON array
    pub provider_filter: Option<String>, // JSON array
    pub rate_limit_per_minute: i32,
    pub enabled: bool,
    pub created_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct CreateVirtualKey {
    pub name: Option<String>,
    pub service_filter: Vec<String>,
    pub provider_filter: Vec<String>,
    pub rate_limit_per_minute: Option<i32>,
}

// Model Health
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ModelHealth {
    pub model_id: String,
    pub status: String, // 'healthy', 'degraded', 'down', 'unknown'
    pub success_rate: f64,
    pub avg_latency_ms: i32,
    pub total_requests: i32,
    pub failed_requests: i32,
    pub last_check: Option<i64>,
}

// Request Log
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RequestLog {
    pub id: String,
    pub virtual_key_id: Option<String>,
    pub service_name: Option<String>,
    pub model_id: Option<String>,
    pub provider_type: Option<String>,
    pub tokens_prompt: Option<i32>,
    pub tokens_completion: Option<i32>,
    pub tokens_total: Option<i32>,
    pub cost_usd: Option<f64>,
    pub latency_ms: Option<i32>,
    pub status: String,
    pub error_message: Option<String>,
    pub failover_count: i32,
    pub created_at: Option<i64>,
}
