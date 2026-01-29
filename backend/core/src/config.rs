use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub models: HashMap<String, ModelConfig>,
    pub quota: QuotaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: String,     // e.g., "openai", "google", "anthropic"
    pub model_id: String,     // e.g., "gpt-4o", "gemini-1.5-pro"
    pub cost_per_token: f64,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaConfig {
    pub daily_limit_usd: f64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            models: HashMap::new(),
            quota: QuotaConfig { daily_limit_usd: 10.0 },
        }
    }
}
