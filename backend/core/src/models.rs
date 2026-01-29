use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// Providers (Azure, OpenAI, Anthropic, Google, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub provider_type: String, // 'azure', 'openai', 'anthropic', 'google'
    pub api_endpoint: Option<String>, // For Azure: base URL like https://my-resource.openai.azure.com
    pub api_version: Option<String>, // For Azure: API version like "2024-12-01-preview"
    #[serde(skip_serializing)] // Don't expose in API responses
    pub api_key: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<i64>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct CreateProvider {
    pub name: String,
    pub provider_type: String,
    pub api_endpoint: Option<String>,
    pub api_version: Option<String>,
    pub api_key: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct UpdateProvider {
    pub name: Option<String>,
    pub provider_type: Option<String>,
    pub api_endpoint: Option<String>,
    pub api_version: Option<String>,
    pub api_key: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
}

// Models (belong to providers)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct Model {
    pub id: String,
    pub name: String,
    #[sqlx(rename = "provider_id")]
    pub provider: String,
    pub modality: String, // 'text', 'audio', 'video'
    pub description: Option<String>,
    
    // Pricing metadata for cost-based routing
    pub cost_per_1k_tokens: Option<f64>,
    pub cost_per_1k_input_tokens: Option<f64>,
    pub cost_per_1k_output_tokens: Option<f64>,
    pub tier: String, // "free", "standard", "premium"
    
    // Performance metrics for latency-based routing
    pub avg_latency_ms: i32,
    pub avg_ttft_ms: i32, // Time to first token
    pub max_tps: i32, // Tokens per second
    
    // Limits
    #[sqlx(default)]
    pub context_window: i32, 
    
    pub api_endpoint: Option<String>,  // Azure: deployment-specific endpoint
    pub api_version: Option<String>,   // Azure: API version
    pub api_key: Option<String>,       // Azure: deployment-specific key
    pub created_at: Option<i64>,
    pub tier_required: String,
    pub worker_type: String,
    pub created_by: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct CreateModel {
    pub name: String,
    pub provider: String,
    pub modality: String,
    pub description: Option<String>,
    
    // Pricing
    pub cost_per_1k_tokens: Option<f64>,
    pub cost_per_1k_input_tokens: Option<f64>,
    pub cost_per_1k_output_tokens: Option<f64>,
    pub tier: Option<String>,
    
    pub api_endpoint: Option<String>,
    pub api_version: Option<String>,
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct UpdateModel {
    pub name: Option<String>,
    pub provider: Option<String>,
    pub modality: Option<String>,
    pub description: Option<String>,
    
    // Pricing
    pub cost_per_1k_tokens: Option<f64>,
    pub cost_per_1k_input_tokens: Option<f64>,
    pub cost_per_1k_output_tokens: Option<f64>,
    pub tier: Option<String>,
    
    pub api_endpoint: Option<String>,
    pub api_version: Option<String>,
    pub api_key: Option<String>,
}
